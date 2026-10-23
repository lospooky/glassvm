#!/usr/bin/env python3
"""Measure the public Python consumer path with isolated provider processes."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import random
import resource
import subprocess
import sys
import tempfile
import time
import tomllib
from datetime import UTC, datetime
from pathlib import Path
from statistics import median

from run_trial import (
    directory_bytes,
    git_metadata,
    machine_configuration,
    observation_request,
)

MODES = (
    "observation_disabled",
    "observed_no_external_sink",
    "python_consumer",
    "rust_recorder",
    "python_consumer_and_rust_recorder",
)


class CountingConsumer:
    """Count callback delivery without retaining emissions or capability data."""

    def __init__(self) -> None:
        self.batch_count = 0
        self.item_count = 0
        self.capability_output_count = 0
        self.capability_receipt_count = 0
        self.run_started: dict | None = None

    def emit_batch(self, emissions: list[dict]) -> None:
        self.batch_count += 1
        self.item_count += len(emissions)
        if self.run_started is None:
            self.run_started = next(
                (
                    item["record"]
                    for item in emissions
                    if item.get("kind") == "run_started"
                ),
                None,
            )

    def record_capability_outputs(self, outputs: list[dict]) -> None:
        self.capability_output_count += len(outputs)

    def record_capability_receipts(self, receipts: list[dict]) -> None:
        self.capability_receipt_count += len(receipts)


def load_workload(paper_root: Path, machines_root: Path, machine_id: str) -> dict:
    data = tomllib.loads((paper_root / "PAPER_WORKLOADS.toml").read_text())
    workload = next(
        (item for item in data["workload"] if item["machine_id"] == machine_id), None
    )
    if workload is None:
        raise ValueError(f"no paper workload is declared for {machine_id!r}")
    artifact_path = machines_root / workload["artifact_path"]
    artifact = artifact_path.read_bytes()
    digest = hashlib.sha256(artifact).hexdigest()
    if digest != workload["artifact_sha256"]:
        raise ValueError(f"fixture hash mismatch for {artifact_path}: {digest}")
    return {**workload, "artifact": artifact, "artifact_digest": digest}


def run_worker(args: argparse.Namespace) -> dict:
    paper_root = args.paper_root.resolve()
    machines_root = args.machines_root.resolve()
    workload = load_workload(paper_root, machines_root, args.machine)
    recipes = tomllib.loads(
        (paper_root / "experiments/OBSERVATION_RECIPES.toml").read_text()
    )["recipe"]
    recipe_id = (
        "disabled" if args.mode == "observation_disabled" else "visual_capability"
    )
    recipe = next(item for item in recipes if item["id"] == recipe_id)
    request_observation = observation_request(recipe, workload)
    schedule = {
        "schema": {
            "id": "glassvm.input_schedule",
            "version": {"major": 1, "minor": 0, "patch": 0},
        },
        "entries": [],
    }
    controls = {"frame_limit": args.frames, "step_limit": None, "bundle_limits": []}
    configuration = machine_configuration(args.machine, workload["configuration"])
    started = datetime.now(UTC).isoformat()
    process_start = time.perf_counter()
    rss_before = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss

    import glassvm

    runtime = glassvm.Runtime.discover()
    providers = json.loads(runtime.bundles())
    provider = next(
        (item for item in providers if item["machine_id"] == args.machine), None
    )
    if provider is None:
        raise RuntimeError(f"no installed provider for {args.machine!r}")
    prepare_start = time.perf_counter()
    prepared = runtime.prepare(
        args.machine,
        workload["artifact"],
        configuration=configuration,
        input_schedule=schedule,
        execution_controls=controls,
        observation=request_observation,
    )
    prepare_seconds = time.perf_counter() - prepare_start
    consumer = (
        CountingConsumer()
        if args.mode in ("python_consumer", "python_consumer_and_rust_recorder")
        else None
    )
    uses_recorder = args.mode in (
        "rust_recorder",
        "python_consumer_and_rust_recorder",
    )

    def execute(file_recorder):
        return prepared.execute(
            sinks=[consumer] if consumer is not None else None,
            file_recorder=file_recorder,
        )

    run_path = None
    run_directory_size = None
    if uses_recorder:
        with tempfile.TemporaryDirectory(
            prefix=f"glassvm-python-sink-{args.machine}-"
        ) as temporary:
            run_path = Path(temporary) / "published-run"
            execute_start = time.perf_counter()
            result = execute(glassvm.FileRecorder(str(run_path)))
            execute_seconds = time.perf_counter() - execute_start
            run_directory_size = (
                directory_bytes(run_path) if run_path.exists() else None
            )
            manifest_path = run_path / "manifest.json"
            recorder_manifest = (
                json.loads(manifest_path.read_text())
                if manifest_path.exists()
                else None
            )
    else:
        execute_start = time.perf_counter()
        result = execute(None)
        execute_seconds = time.perf_counter() - execute_start
        recorder_manifest = None

    if not isinstance(result, dict):
        raise TypeError("PreparedRun.execute did not return a structured mapping")
    recorder_outcome = result["recorder_finalization"]
    recorder_receipt = (
        recorder_outcome.get("result")
        if recorder_outcome.get("status") == "finalized"
        else None
    )
    sink_outcome = result["python_sink_delivery"]
    if consumer is not None and sink_outcome["status"] != "complete":
        raise RuntimeError(f"Python consumer delivery failed: {sink_outcome}")
    if uses_recorder and result["publication"]["status"] != "published":
        raise RuntimeError(f"reference recorder did not publish: {result}")

    peak_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return {
        "schema": "glassvm.paper_python_sink_trial",
        "schema_version": "1.0.0",
        "trial": {
            "machine_id": args.machine,
            "mode": args.mode,
            "repetition": args.repetition,
            "frame_limit": args.frames,
            "started_utc": started,
        },
        "source": {
            "glassvm": git_metadata(args.glassvm_root.resolve()),
            "machines": git_metadata(machines_root),
            "provider": provider,
            "artifact_sha256": workload["artifact_digest"],
        },
        "request": {
            "configuration": configuration,
            "input_schedule": schedule,
            "execution_controls": controls,
            "observation": request_observation,
        },
        "identity": {
            "run_id": (
                consumer.run_started.get("run_id")
                if consumer is not None and consumer.run_started
                else (recorder_manifest or {}).get("run_id")
            ),
            "prepared_observation_id": (
                consumer.run_started.get("prepared_observation_id")
                if consumer is not None and consumer.run_started
                else (recorder_manifest or {}).get("prepared_observation_id")
            ),
        },
        "execution": result["machine_execution"],
        "evidence_fulfillment": result["evidence_fulfillment"],
        "python_sink_delivery": sink_outcome,
        "python_consumer_counts": (
            {
                "batches": consumer.batch_count,
                "items": consumer.item_count,
                "capability_outputs": consumer.capability_output_count,
                "capability_receipts": consumer.capability_receipt_count,
            }
            if consumer is not None
            else None
        ),
        "recorder_finalization": recorder_outcome,
        "recorder_receipt": recorder_receipt,
        "publication": {
            "status": result["publication"]["status"],
            "published": result["published_run"] is not None,
            "run_directory_bytes": run_directory_size,
        },
        "timing": {
            "preparation_seconds": prepare_seconds,
            "execute_lifecycle_seconds": execute_seconds,
            "process_seconds": time.perf_counter() - process_start,
        },
        "memory": {
            "peak_rss_bytes": int(peak_rss * 1024)
            if platform.system() == "Linux"
            else None,
            "peak_rss_native_value": peak_rss,
            "peak_rss_native_unit": "KiB" if platform.system() == "Linux" else None,
            "rss_before_native_value": rss_before,
            "allocator_high_water_bytes": None,
        },
    }


def summarize(trials: list[dict]) -> dict:
    summary = {}
    for machine in sorted({row["trial"]["machine_id"] for row in trials}):
        selected = [row for row in trials if row["trial"]["machine_id"] == machine]
        by_repetition: dict[int, dict[str, dict]] = {}
        for row in selected:
            by_repetition.setdefault(row["trial"]["repetition"], {})[
                row["trial"]["mode"]
            ] = row
        for repetition, modes in by_repetition.items():
            observed = {
                name: row
                for name, row in modes.items()
                if name != "observation_disabled"
            }
            reference = modes.get("observed_no_external_sink")
            if reference is None:
                raise ValueError(
                    f"{machine} repetition {repetition} lacks the observed reference"
                )
            for name, row in modes.items():
                machine_result = row["execution"].get("result", {})
                reference_result = reference["execution"].get("result", {})
                if row["execution"].get("status") != reference["execution"].get(
                    "status"
                ) or machine_result.get("common") != reference_result.get("common"):
                    raise ValueError(
                        f"{machine} repetition {repetition} execution differs in {name}"
                    )
            for name, row in observed.items():
                machine_result = row["execution"].get("result", {})
                reference_result = reference["execution"].get("result", {})
                if row["request"] != reference["request"]:
                    raise ValueError(
                        f"{machine} repetition {repetition} observed request differs in {name}"
                    )
                if row["evidence_fulfillment"] != reference["evidence_fulfillment"]:
                    raise ValueError(
                        f"{machine} repetition {repetition} evidence receipt differs in {name}"
                    )
                if machine_result.get("capabilities") != reference_result.get(
                    "capabilities"
                ):
                    raise ValueError(
                        f"{machine} repetition {repetition} capability output differs in {name}"
                    )
            observation_ids = {
                json.dumps(row["identity"]["prepared_observation_id"], sort_keys=True)
                for row in observed.values()
                if row["identity"]["prepared_observation_id"] is not None
            }
            if len(observation_ids) > 1:
                raise ValueError(
                    f"{machine} repetition {repetition} prepared-observation identity differs"
                )
            run_ids = {
                row["identity"]["run_id"]
                for row in modes.values()
                if row["identity"]["run_id"] is not None
            }
            if len(run_ids) > 1:
                raise ValueError(
                    f"{machine} repetition {repetition} run identity differs"
                )
        reference = median(
            row["timing"]["execute_lifecycle_seconds"]
            for row in selected
            if row["trial"]["mode"] == "observed_no_external_sink"
        )
        summary[machine] = {}
        for mode in MODES:
            values = [
                row["timing"]["execute_lifecycle_seconds"]
                for row in selected
                if row["trial"]["mode"] == mode
            ]
            if not values:
                continue
            current = median(values)
            summary[machine][mode] = {
                "median_execute_lifecycle_seconds": current,
                "median_delta_vs_observed_no_external_sink_seconds": current
                - reference,
                "relative_delta_percent_vs_observed_no_external_sink": (
                    (current - reference) / reference * 100 if reference else None
                ),
            }
    return summary


def load_existing_trials(
    output_dir: Path, machines: list[str], modes: list[str], frames: int, repetitions: int
) -> list[dict]:
    raw_dir = output_dir / "raw"
    trials = [
        json.loads(path.read_text())
        for path in sorted(raw_dir.glob("*.json"))
    ]
    expected = {
        (machine, mode, repetition)
        for machine in machines
        for mode in modes
        for repetition in range(1, repetitions + 1)
    }
    actual = [
        (
            trial["trial"]["machine_id"],
            trial["trial"]["mode"],
            trial["trial"]["repetition"],
        )
        for trial in trials
    ]
    if len(actual) != len(set(actual)):
        raise ValueError("existing raw trials contain duplicate matrix coordinates")
    if set(actual) != expected:
        missing = sorted(expected - set(actual))
        unexpected = sorted(set(actual) - expected)
        raise ValueError(
            f"existing raw trial matrix is incomplete or mismatched; "
            f"missing={missing}, unexpected={unexpected}"
        )
    wrong_frames = [
        trial["trial"]
        for trial in trials
        if trial["trial"]["frame_limit"] != frames
    ]
    if wrong_frames:
        raise ValueError(f"existing raw trials have a different frame limit: {wrong_frames}")
    return trials


def write_results(args: argparse.Namespace, trials: list[dict]) -> None:
    summary = summarize(trials)
    source = {
        name: trials[0]["source"][name] for name in ("glassvm", "machines")
    }
    for trial in trials[1:]:
        if any(trial["source"][name] != source[name] for name in source):
            raise ValueError("raw trials were collected from different source coordinates")
    manifest = {
        "schema": "glassvm.paper_python_sink_manifest",
        "schema_version": "1.0.0",
        "created_utc": datetime.now(UTC).isoformat(),
        "policy": {
            "independent_process_per_trial": True,
            "frame_limit": args.frames,
            "repetitions": args.repetitions,
            "modes": args.modes,
            "machines": args.machines,
            "order_seed": args.order_seed,
            "consumer_retains_emissions": False,
        },
        "host": {
            "system": platform.platform(),
            "architecture": platform.machine(),
            "python": sys.version,
        },
        "source": source,
        "trial_count": len(trials),
    }
    (args.output_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    (args.output_dir / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n"
    )
    lines = [
        "# Python consumer sink-cost experiment",
        "",
        "This is a host/workload-specific end-to-end lifecycle comparison. The",
        "`observed_no_external_sink` row is not a direct `NullSink` microbenchmark;",
        "the public provider lifecycle still applies the shared budgeted bridge.",
        "Recorder rows include finalization and atomic publication.",
        "",
        "| Machine | Mode | Median lifecycle (s) | Delta vs observed/no external sink (s) | Delta (%) |",
        "|---|---|---:|---:|---:|",
    ]
    for machine, rows in summary.items():
        for mode, values in rows.items():
            lines.append(
                f"| {machine} | {mode} | {values['median_execute_lifecycle_seconds']:.3f} | "
                f"{values['median_delta_vs_observed_no_external_sink_seconds']:.3f} | "
                f"{values['relative_delta_percent_vs_observed_no_external_sink']:.1f}% |"
            )
    lines.extend(
        [
            "",
            "Raw trials record per-run batch/item/logical-byte counts, peak bounded",
            "batch size, RSS high-water where available, receipts, provider metadata,",
            "and both checkout revisions. Regenerate from clean pinned builds before",
            "using these values as paper evidence.",
            "",
        ]
    )
    (args.output_dir / "SUMMARY.md").write_text("\n".join(lines))
    print(f"Wrote Python sink-cost results to {args.output_dir}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--paper-root", type=Path, default=Path(__file__).resolve().parents[1]
    )
    parser.add_argument(
        "--machines-root",
        type=Path,
        default=Path(__file__).resolve().parents[4] / "glassvm-machines",
    )
    parser.add_argument(
        "--glassvm-root", type=Path, default=Path(__file__).resolve().parents[3]
    )
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--frames", type=int, default=60_000)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--machines", nargs="*", default=["chip8", "pico8", "tic80"])
    parser.add_argument("--modes", nargs="*", choices=MODES, default=list(MODES))
    parser.add_argument("--order-seed", type=int, default=20261011)
    parser.add_argument("--worker", action="store_true")
    parser.add_argument(
        "--summarize-existing",
        action="store_true",
        help="validate and summarize a complete existing raw-trial matrix",
    )
    parser.add_argument("--machine")
    parser.add_argument("--mode", choices=MODES)
    parser.add_argument("--repetition", type=int)
    args = parser.parse_args()
    if args.worker:
        trial = run_worker(args)
        print(json.dumps(trial, sort_keys=True))
        return
    if args.frames < 1 or args.repetitions < 1:
        parser.error("--frames and --repetitions must be positive")
    if "observed_no_external_sink" not in args.modes:
        parser.error(
            "--modes must include observed_no_external_sink as the comparison reference"
        )
    args.output_dir.mkdir(parents=True, exist_ok=True)
    if args.summarize_existing:
        trials = load_existing_trials(
            args.output_dir, args.machines, args.modes, args.frames, args.repetitions
        )
        write_results(args, trials)
        return
    cases = [
        (machine, mode, repetition)
        for machine in args.machines
        for mode in args.modes
        for repetition in range(1, args.repetitions + 1)
    ]
    random.Random(args.order_seed).shuffle(cases)
    trials = []
    for index, (machine, mode, repetition) in enumerate(cases, start=1):
        name = f"{machine}__{mode}__{args.frames}__r{repetition}.json"
        output = args.output_dir / "raw" / name
        output.parent.mkdir(parents=True, exist_ok=True)
        command = [
            sys.executable,
            str(Path(__file__).resolve()),
            "--worker",
            "--paper-root",
            str(args.paper_root.resolve()),
            "--machines-root",
            str(args.machines_root.resolve()),
            "--glassvm-root",
            str(args.glassvm_root.resolve()),
            "--output-dir",
            str(args.output_dir.resolve()),
            "--frames",
            str(args.frames),
            "--machine",
            machine,
            "--mode",
            mode,
            "--repetition",
            str(repetition),
        ]
        started = time.perf_counter()
        process = subprocess.run(command, check=False, capture_output=True, text=True)
        if process.returncode:
            raise RuntimeError(
                f"trial {name} failed ({process.returncode}):\n{process.stderr}"
            )
        trial = json.loads(process.stdout)
        output.write_text(json.dumps(trial, indent=2, sort_keys=True) + "\n")
        trials.append(trial)
        print(
            json.dumps(
                {
                    "index": index,
                    "total": len(cases),
                    "trial": name,
                    "seconds": round(time.perf_counter() - started, 3),
                }
            ),
            flush=True,
        )
    write_results(args, trials)


if __name__ == "__main__":
    main()
