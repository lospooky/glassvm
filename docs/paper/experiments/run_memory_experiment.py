#!/usr/bin/env python3
"""Measure memory and evidence-volume scaling across isolated GlassVM runs."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import random
import subprocess
import sys
import tempfile
import time
import tomllib
from datetime import datetime, timezone
from pathlib import Path

from run_trial import machine_configuration

MACHINES = ("chip8", "pico8", "tic80")
MODES = ("disabled", "events_null", "normalizer_null", "recorder")
DEFAULT_LENGTHS = (600, 6_000, 60_000, 600_000)
DEFAULT_REPETITIONS = {600: 3, 6_000: 3, 60_000: 3, 600_000: 1}


def load_manifest(paper_root: Path, machines_root: Path) -> dict[str, dict]:
    manifest = tomllib.loads((paper_root / "PAPER_WORKLOADS.toml").read_text())
    result = {}
    for workload in manifest["workload"]:
        if workload["machine_id"] not in MACHINES:
            continue
        path = machines_root / workload["artifact_path"]
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != workload["artifact_sha256"]:
            raise ValueError(f"fixture hash mismatch for {path}: {digest}")
        result[workload["machine_id"]] = workload
    missing = set(MACHINES) - result.keys()
    if missing:
        raise ValueError(f"workload manifest missing machines: {sorted(missing)}")
    return result


def git_value(path: Path, *args: str) -> str | None:
    try:
        return subprocess.check_output(
            ["git", "-C", str(path), *args], text=True, stderr=subprocess.DEVNULL
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def git_source(path: Path) -> dict[str, object]:
    status = git_value(path, "status", "--porcelain", "--untracked-files=all")
    return {
        "revision": git_value(path, "rev-parse", "HEAD"),
        "exact_tag": git_value(path, "describe", "--tags", "--exact-match"),
        "dirty": status != "",
    }


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def build_trial_binary(conformance_root: Path) -> Path:
    completed = subprocess.run(
        [
            "cargo",
            "build",
            "--release",
            "--locked",
            "--example",
            "paper_memory_trial",
            "--message-format=json",
        ],
        cwd=conformance_root,
        text=True,
        capture_output=True,
        check=False,
    )
    artifacts = []
    diagnostics = []
    for line in completed.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("reason") == "compiler-message":
            rendered = message.get("message", {}).get("rendered")
            if rendered:
                diagnostics.append(rendered)
        if message.get("reason") != "compiler-artifact":
            continue
        target = message.get("target", {})
        executable = message.get("executable")
        if (
            target.get("name") == "paper_memory_trial"
            and "example" in target.get("kind", [])
            and executable
        ):
            artifacts.append(Path(executable).resolve())

    if completed.returncode != 0:
        if completed.stderr:
            sys.stderr.write(completed.stderr)
        for diagnostic in diagnostics:
            sys.stderr.write(diagnostic)
        raise RuntimeError(
            f"Cargo failed to build paper_memory_trial (exit {completed.returncode})"
        )
    artifacts = list(dict.fromkeys(artifacts))
    if len(artifacts) != 1:
        raise RuntimeError(
            "Cargo did not report exactly one paper_memory_trial executable "
            f"(found {len(artifacts)})"
        )
    binary = artifacts[0]
    if not binary.is_file():
        raise FileNotFoundError(f"Cargo-reported trial executable does not exist: {binary}")
    return binary


def typed_request_value(mode: str, workload: dict) -> tuple[list[object], str]:
    event_kinds = [
        {"extension": value.removeprefix("extension:")}
        if value.startswith("extension:")
        else value
        for value in workload["normalized_event_kinds"]
    ]
    capability = workload["capability"]
    if mode in ("disabled", "events_null"):
        event_kinds = [] if mode == "disabled" else event_kinds
    return event_kinds, capability


def trial_command(
    binary: Path,
    machines_root: Path,
    workload: dict,
    mode: str,
    frame_limit: int,
    repetition: int,
    recorder_path: Path | None,
) -> list[str]:
    config = machine_configuration(workload["machine_id"], workload["configuration"])
    event_kinds, capability = typed_request_value(mode, workload)
    command = [
        str(binary),
        "--machine", workload["machine_id"],
        "--mode", mode,
        "--frame-limit", str(frame_limit),
        "--repetition", str(repetition),
        "--artifact", str(machines_root / workload["artifact_path"]),
        "--configuration-json", json.dumps(config, separators=(",", ":")),
        "--event-kinds-json", json.dumps(event_kinds, separators=(",", ":")),
        "--capability", capability,
    ]
    if recorder_path is not None:
        command.extend(("--output-path", str(recorder_path)))
    return command


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--paper-root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--machines-root", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--frames", type=int, nargs="+")
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--machine", choices=MACHINES, action="append")
    parser.add_argument("--mode", choices=MODES, action="append")
    parser.add_argument("--seed", type=int, default=20261009)
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--require-clean", action="store_true")
    parser.add_argument("--expected-glassvm-revision")
    parser.add_argument("--expected-machines-revision")
    args = parser.parse_args()

    paper_root = args.paper_root.resolve()
    machines_root = args.machines_root.resolve()
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    conformance_root = machines_root / "conformance"
    glassvm_root = paper_root.parents[1]
    glassvm_source = git_source(glassvm_root)
    machines_source = git_source(machines_root)
    if args.require_clean:
        if args.skip_build:
            raise ValueError("--require-clean requires building the trial binary in this invocation")
        if not args.expected_glassvm_revision or not args.expected_machines_revision:
            raise ValueError(
                "--require-clean requires --expected-glassvm-revision and "
                "--expected-machines-revision"
            )
        if glassvm_source["dirty"] or machines_source["dirty"]:
            raise ValueError("--require-clean needs clean GlassVM and glassvm-machines checkouts")
        for name, source, expected, root in (
            ("GlassVM", glassvm_source, args.expected_glassvm_revision, glassvm_root),
            ("glassvm-machines", machines_source, args.expected_machines_revision, machines_root),
        ):
            expected_revision = git_value(root, "rev-parse", f"{expected}^{{commit}}")
            if not expected_revision or expected_revision != source["revision"]:
                raise ValueError(
                    f"{name} HEAD {source['revision']} does not match expected revision {expected}"
                )
        for label, output in (("GlassVM", glassvm_root), ("glassvm-machines", machines_root)):
            try:
                output_dir.relative_to(output)
            except ValueError:
                continue
            raise ValueError(
                f"--require-clean output directory must be outside the {label} checkout"
            )
    if args.skip_build:
        if args.binary is None:
            raise ValueError("--skip-build requires an explicit --binary path")
        binary = args.binary.resolve()
    else:
        if args.binary is not None:
            raise ValueError("--binary may only be used together with --skip-build")
        binary = build_trial_binary(conformance_root)
    if not binary.is_file():
        raise FileNotFoundError(f"trial binary does not exist: {binary}")

    if args.require_clean:
        # Building may create ignored files, but must not change tracked or untracked source.
        if git_source(glassvm_root)["dirty"] or git_source(machines_root)["dirty"]:
            raise ValueError("a checkout became dirty while building the trial binary")

    workloads = load_manifest(paper_root, machines_root)
    lengths = tuple(args.frames or DEFAULT_LENGTHS)
    machines = tuple(args.machine or MACHINES)
    modes = tuple(args.mode or MODES)
    jobs = []
    for machine in machines:
        for mode in modes:
            for frames in lengths:
                repetitions = args.repetitions if args.frames else DEFAULT_REPETITIONS.get(frames, 3)
                for repetition in range(1, repetitions + 1):
                    jobs.append((machine, mode, frames, repetition))
    random.Random(args.seed).shuffle(jobs)

    metadata_path = output_dir / "experiment.json"
    prior_metadata = json.loads(metadata_path.read_text()) if metadata_path.exists() else {}
    if not metadata_path.exists() and any(output_dir.glob("*__*.json")):
        raise ValueError(
            "output directory has trial files but no experiment manifest; "
            "use a fresh output directory"
        )
    metadata = {
        "schema": "glassvm.paper_memory_experiment",
        "schema_version": "1.0.0",
        "started_utc": prior_metadata.get("started_utc", datetime.now(timezone.utc).isoformat()),
        "machines": machines,
        "modes": modes,
        "frame_limits": lengths,
        "repetitions": {str(length): args.repetitions if args.frames else DEFAULT_REPETITIONS.get(length, 3) for length in lengths},
        "randomized_job_order_seed": args.seed,
        "source": {
            "glassvm": glassvm_source,
            "machines": machines_source,
            "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
            "cargo": subprocess.check_output(["cargo", "--version"], text=True).strip(),
            "machines_cargo_lock_sha256": file_sha256(machines_root / "Cargo.lock"),
            "trial_binary_sha256": file_sha256(binary),
            "trial_binary_origin": (
                "explicit_binary" if args.skip_build else "cargo_compiler_artifact"
            ),
            "host_system": platform.platform(),
            "host_machine": platform.machine(),
            "python": sys.version,
        },
        "workloads": {
            machine: {
                "artifact_path": workload["artifact_path"],
                "artifact_sha256": workload["artifact_sha256"],
                "configuration": workload["configuration"],
                "input_schedule": "empty",
                "normalized_event_kinds": workload["normalized_event_kinds"],
                "capability": workload["capability"],
            }
            for machine, workload in workloads.items()
        },
        "recorder_limits": {
            "max_block_logical_bytes": 1_048_576,
            "max_segment_records": 256,
            "max_segment_logical_bytes": 2_097_152,
            "max_buffered_bytes_per_channel": 1_048_576,
        },
    }
    immutable_keys = (
        "schema",
        "schema_version",
        "machines",
        "modes",
        "frame_limits",
        "repetitions",
        "randomized_job_order_seed",
        "source",
        "workloads",
        "recorder_limits",
    )
    if prior_metadata:
        mismatches = [
            key for key in immutable_keys if prior_metadata.get(key) != metadata.get(key)
        ]
        if mismatches:
            raise ValueError(
                "existing output belongs to a different source or trial matrix; "
                f"use a fresh output directory (mismatches: {', '.join(mismatches)})"
            )
    if prior_metadata:
        metadata["started_utc"] = prior_metadata.get(
            "started_utc", metadata["started_utc"]
        )
        metadata["resumed_utc"] = datetime.now(timezone.utc).isoformat()

    expected_paths = {
        output_dir / f"{machine}__{mode}__{frames}__r{repetition}.json"
        for machine, mode, frames, repetition in jobs
    }
    stale_paths = {
        path
        for path in output_dir.glob("*__*.json")
        if path.name != "experiment.json" and path not in expected_paths
    }
    if stale_paths:
        raise ValueError(
            "output directory contains trial files outside this matrix; use a fresh "
            f"output directory: {', '.join(sorted(path.name for path in stale_paths))}"
        )
    metadata_path.write_text(json.dumps(metadata, indent=2) + "\n")

    failures = 0
    for index, (machine, mode, frames, repetition) in enumerate(jobs, start=1):
        result_path = output_dir / f"{machine}__{mode}__{frames}__r{repetition}.json"
        if result_path.exists():
            previous = json.loads(result_path.read_text())
            previous_trial = previous.get("trial", {})
            expected_trial = {
                "machine_id": machine,
                "mode": mode,
                "frame_limit": frames,
                "repetition": repetition,
            }
            if previous.get("status") == "completed":
                actual_trial = {key: previous_trial.get(key) for key in expected_trial}
                if actual_trial != expected_trial:
                    raise ValueError(f"trial file identity mismatch: {result_path.name}")
                continue
        run_dir = None
        try:
            if mode == "recorder":
                run_dir = tempfile.TemporaryDirectory(prefix="glassvm-paper-memory-")
                recorder_path = Path(run_dir.name) / "published-run"
            else:
                recorder_path = None
            command = trial_command(
                binary, machines_root, workloads[machine], mode, frames, repetition, recorder_path
            )
            started = time.perf_counter()
            completed = subprocess.run(command, text=True, capture_output=True, check=False)
            wall_seconds = time.perf_counter() - started
            if completed.returncode == 0:
                try:
                    result = json.loads(completed.stdout)
                    result["measurement"] = {"process_wall_seconds": wall_seconds}
                    result["status"] = "completed"
                except json.JSONDecodeError as error:
                    failures += 1
                    result = {"status": "invalid_output", "error": str(error), "stdout": completed.stdout}
            else:
                failures += 1
                result = {
                    "status": "failed",
                    "exit_code": completed.returncode,
                    "stderr": completed.stderr,
                    "stdout": completed.stdout,
                    "measurement": {"process_wall_seconds": wall_seconds},
                    "trial": {"machine_id": machine, "mode": mode, "frame_limit": frames, "repetition": repetition},
                }
            result_path.write_text(json.dumps(result, indent=2) + "\n")
            print(f"[{index}/{len(jobs)}] {machine} {mode} {frames} r{repetition}: {result['status']} ({wall_seconds:.2f}s)", flush=True)
        finally:
            if run_dir is not None:
                run_dir.cleanup()

    metadata["finished_utc"] = datetime.now(timezone.utc).isoformat()
    metadata["completed_trials"] = sum(
        1
        for path in expected_paths
        if path.exists() and json.loads(path.read_text()).get("status") == "completed"
    )
    failures = sum(
        1
        for path in expected_paths
        if path.exists() and json.loads(path.read_text()).get("status") != "completed"
    )
    metadata["failed_trials"] = failures
    (output_dir / "experiment.json").write_text(json.dumps(metadata, indent=2) + "\n")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
