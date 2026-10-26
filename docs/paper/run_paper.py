#!/usr/bin/env python3
"""Single entry point for preparing and regenerating the GlassVM paper artifacts.

Run with ``uv`` and CPython 3.12+. The entry point creates a disposable locked
Python environment and isolated Cargo home/target for each invocation. Python
collections use published 0.3.0 distributions; Rust contract dependencies are
resolved from crates.io, while paper-only machine trials are built from the
companion machine checkout.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import os
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # The bootstrap host may predate Python 3.11.
    tomllib = None

UTC = timezone.utc


PAPER_ROOT = Path(__file__).resolve().parent
GLASSVM_ROOT = PAPER_ROOT.parents[1]
DEFAULT_MACHINES_ROOT = GLASSVM_ROOT.parent / "glassvm-machines"
DEFAULT_RESULTS_ROOT = GLASSVM_ROOT.parent / "glassvm-paper-results" / "0.3.0-final"
VERSION = "0.3.0"
MACHINES = ("chip8", "pico8", "tic80")
PROVIDER_DISTRIBUTIONS = (
    "glassvm-chip8",
    "glassvm-pico8",
    "glassvm-tic80",
)
ALL_DISTRIBUTIONS = ("glassvm", *PROVIDER_DISTRIBUTIONS, "glassvm-machines")
EXPERIMENTS = PAPER_ROOT / "experiments"


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(root), *args], text=True, stderr=subprocess.DEVNULL
    ).strip()


def source_record(root: Path) -> dict[str, object]:
    return {
        "revision": git(root, "rev-parse", "HEAD"),
        "branch": git(root, "branch", "--show-current"),
        "dirty": bool(git(root, "status", "--porcelain", "--untracked-files=all")),
        "release_tag": git(root, "describe", "--tags", "--exact-match")
        if subprocess.run(
            ["git", "-C", str(root), "describe", "--tags", "--exact-match"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        ).returncode
        == 0
        else None,
    }


def require_repo(root: Path, label: str) -> None:
    if not (root / ".git").exists() and not (root / ".git").is_file():
        raise RuntimeError(f"{label} is not a Git checkout: {root}")


def require_clean(root: Path, label: str) -> None:
    if git(root, "status", "--porcelain", "--untracked-files=all"):
        raise RuntimeError(
            f"{label} checkout is dirty; final paper measurements require clean sources: {root}"
        )


def require_release_packages() -> None:
    actual: dict[str, str] = {}
    for distribution in ALL_DISTRIBUTIONS:
        try:
            actual[distribution] = importlib.metadata.version(distribution)
        except importlib.metadata.PackageNotFoundError as error:
            raise RuntimeError(
                f"missing {distribution}=={VERSION} in the clean-room environment; check the locked paper dependencies"
            ) from error
    mismatched = {
        name: version for name, version in actual.items() if version != VERSION
    }
    if mismatched:
        raise RuntimeError(
            f"paper runs require all distributions at {VERSION}; found {mismatched}"
        )

    import glassvm

    providers = {
        item["machine_id"] for item in json.loads(glassvm.Runtime.discover().bundles())
    }
    missing = set(MACHINES) - providers
    if missing:
        raise RuntimeError(
            "installed 0.3.0 providers do not expose all paper machines: "
            + ", ".join(sorted(missing))
        )


def command(label: str, argv: list[str], cwd: Path) -> None:
    print(f"\n== {label} ==\n$ {' '.join(map(str, argv))}", flush=True)
    subprocess.run(argv, cwd=cwd, check=True)


def output_of(argv: list[str]) -> str:
    return subprocess.check_output(argv, text=True).strip()


def run_external_rust_consumer() -> None:
    project = EXPERIMENTS / "external-consumer-proof/rust/Cargo.toml"
    command(
        "build and run standalone Rust consumer using published crates.io dependencies",
        ["cargo", "run", "--locked", "--manifest-path", str(project)],
        GLASSVM_ROOT,
    )


def verify_fixtures(machines_root: Path) -> None:
    if tomllib is None:
        raise RuntimeError("paper stages require the isolated Python 3.12 environment")
    manifest = tomllib.loads((PAPER_ROOT / "PAPER_WORKLOADS.toml").read_text())
    workloads = manifest["workload"]
    found = {item["machine_id"] for item in workloads}
    if found != set(MACHINES):
        raise RuntimeError(f"paper workload manifest machine set is {sorted(found)}")

    with tempfile.TemporaryDirectory(prefix="glassvm-paper-fixtures-") as temporary:
        temp_root = Path(temporary)
        for workload in workloads:
            machine = workload["machine_id"]
            fixture_dir = machines_root / machine / "fixtures"
            artifact = machines_root / workload["artifact_path"]
            expected = workload["artifact_sha256"]
            checked_in_digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
            if checked_in_digest != expected:
                raise RuntimeError(
                    f"checked-in {machine} paper fixture digest mismatch: {checked_in_digest}"
                )

            rebuild = workload["rebuild_command"]
            if len(rebuild) < 4 or rebuild[-2] != "--output":
                raise RuntimeError(f"unsupported rebuild command for {machine}: {rebuild}")
            script = fixture_dir / rebuild[1]
            generated = temp_root / machine / Path(rebuild[-1]).name
            generated.parent.mkdir(parents=True, exist_ok=True)
            command(
                f"rebuild/verify {machine} paper fixture",
                [sys.executable, str(script), "--output", str(generated)],
                fixture_dir,
            )
            generated_digest = hashlib.sha256(generated.read_bytes()).hexdigest()
            if generated_digest != expected:
                raise RuntimeError(
                    f"rebuilt {machine} paper fixture digest mismatch: {generated_digest}"
                )
            print(f"{machine}: SHA-256 {expected} verified", flush=True)


def stage_check(args: argparse.Namespace) -> None:
    require_repo(args.glassvm_root, "GlassVM")
    require_repo(args.machines_root, "glassvm-machines")
    require_release_packages()
    run_external_rust_consumer()
    verify_fixtures(args.machines_root)
    command(
        "paper harness unit tests",
        [
            sys.executable,
            "-m",
            "unittest",
            "discover",
            "-s",
            str(EXPERIMENTS),
            "-p",
            "test_*.py",
        ],
        GLASSVM_ROOT,
    )
    command(
        "compile the paper-only machine memory trial against the locked 0.3.0 contracts",
        [
            "cargo",
            "check",
            "--locked",
            "--manifest-path",
            str(args.machines_root / "conformance/Cargo.toml"),
            "--example",
            "paper_memory_trial",
        ],
        args.machines_root,
    )
    command(
        "check generated heterogeneity tables",
        [sys.executable, str(PAPER_ROOT / "generate_heterogeneity.py"), "--check"],
        GLASSVM_ROOT,
    )


def run_pilot(args: argparse.Namespace, output: Path, repetitions: int) -> None:
    pilot_argv = [
        sys.executable,
        str(EXPERIMENTS / "run_pilot.py"),
        "--glassvm-root",
        str(args.glassvm_root),
        "--machines-root",
        str(args.machines_root),
        "--output-dir",
        str(output),
        "--repetitions",
        str(repetitions),
        "--frame-limit",
        "600",
    ]
    if repetitions > 1:
        pilot_argv.extend(["--manifest-name", "measurement_manifest.json"])
    command(
        f"published-wheel observation matrix ({repetitions} repetitions)",
        pilot_argv,
        GLASSVM_ROOT,
    )
    if repetitions == 1:
        summary_script = "generate_pilot_summary.py"
        summary_args = ["--pilot-dir", str(output)]
    else:
        summary_script = "generate_observation_summary.py"
        summary_args = ["--measurement-dir", str(output)]
    command(
        "render observation matrix summary",
        [sys.executable, str(EXPERIMENTS / summary_script), *summary_args],
        GLASSVM_ROOT,
    )


def run_memory(args: argparse.Namespace, output: Path, sink_only: bool = False) -> None:
    expected_glassvm = git(args.glassvm_root, "rev-parse", "HEAD")
    expected_machines = git(args.machines_root, "rev-parse", "HEAD")
    argv = [
        sys.executable,
        str(EXPERIMENTS / "run_memory_experiment.py"),
        "--paper-root",
        str(PAPER_ROOT),
        "--machines-root",
        str(args.machines_root),
        "--output-dir",
        str(output),
        "--require-clean",
        "--expected-glassvm-revision",
        expected_glassvm,
        "--expected-machines-revision",
        expected_machines,
    ]
    if sink_only:
        argv.extend(
            [
                "--frames",
                "60000",
                "--repetitions",
                "3",
                "--mode",
                "disabled",
                "--mode",
                "normalizer_null",
                "--mode",
                "recorder",
            ]
        )
    command("Rust memory/sink experiment", argv, GLASSVM_ROOT)
    summarizer = "summarize_sink_cost.py" if sink_only else "summarize_memory_experiment.py"
    command(
        "summarize Rust memory/sink experiment",
        [sys.executable, str(EXPERIMENTS / summarizer), "--data-dir", str(output)],
        GLASSVM_ROOT,
    )


def run_python_sink(args: argparse.Namespace, output: Path) -> None:
    command(
        "Python consumer sink-cost matrix",
        [
            sys.executable,
            str(EXPERIMENTS / "run_python_sink_cost.py"),
            "--paper-root",
            str(PAPER_ROOT),
            "--machines-root",
            str(args.machines_root),
            "--glassvm-root",
            str(args.glassvm_root),
            "--output-dir",
            str(output),
            "--frames",
            "60000",
            "--repetitions",
            "3",
        ],
        GLASSVM_ROOT,
    )


def run_semantic_equivalence(args: argparse.Namespace, output: Path) -> None:
    command(
        "Python/Rust fan-out semantic-equivalence matrix",
        [
            sys.executable,
            str(EXPERIMENTS / "run_semantic_equivalence.py"),
            "--glassvm-root",
            str(args.glassvm_root),
            "--machines-root",
            str(args.machines_root),
            "--output",
            str(output),
            "--frames",
            "600",
        ],
        GLASSVM_ROOT,
    )


def run_garden(args: argparse.Namespace, output: Path) -> None:
    command(
        "Glass Garden consumer postcards",
        [
            sys.executable,
            str(EXPERIMENTS / "glass_garden.py"),
            "--machines-root",
            str(args.machines_root),
            "--output-dir",
            str(output),
        ],
        GLASSVM_ROOT,
    )


def run_relay(args: argparse.Namespace, output: Path) -> None:
    command(
        "Glass Relay cross-machine behavioral comparison",
        [sys.executable, str(EXPERIMENTS / "glass_relay.py"),
         "--machines-root", str(args.machines_root), "--output-dir", str(output)],
        GLASSVM_ROOT,
    )


def run_external_consumer(args: argparse.Namespace, output: Path) -> None:
    command(
        "clean-room third-party consumer matrix",
        [
            "bash",
            str(EXPERIMENTS / "external-consumer-proof/python/run_matrix.sh"),
            "--output-root",
            str(output),
        ],
        GLASSVM_ROOT,
    )


def run_all(args: argparse.Namespace) -> None:
    require_clean(args.glassvm_root, "GlassVM")
    require_clean(args.machines_root, "glassvm-machines")
    results = args.results_root
    if results.exists() and any(results.iterdir()):
        raise RuntimeError(
            f"refusing to overwrite non-empty result directory: {results}; choose a new --results-root"
        )
    results.mkdir(parents=True, exist_ok=True)
    manifest_path = results / "paper-run.json"
    manifest = {
        "schema": "glassvm.paper_run",
        "schema_version": "1.0.0",
        "release_version": VERSION,
        "started_at_utc": datetime.now(UTC).isoformat(),
        "python": sys.version,
        "toolchain": {
            "uv": output_of(["uv", "--version"]),
            "cargo": output_of(["cargo", "--version"]),
            "rustc": output_of(["rustc", "--version"]),
            "python_distributions": {
                name: importlib.metadata.version(name) for name in ALL_DISTRIBUTIONS
            },
            "environment_policy": "fresh temporary uv environment, UV cache disabled, isolated CARGO_HOME and CARGO_TARGET_DIR",
        },
        "glassvm": source_record(args.glassvm_root),
        "machines": source_record(args.machines_root),
        "stages": [],
    }
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")

    def run_stage(name: str, callback) -> None:
        entry = {"name": name, "status": "running"}
        manifest["stages"].append(entry)
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
        try:
            callback()
        except Exception as error:
            entry["status"] = "failed"
            entry["error"] = str(error)
            manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
            raise
        entry["status"] = "complete"
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")

    run_stage("preflight", lambda: stage_check(args))
    run_stage("glass-garden", lambda: run_garden(args, results / "glass-garden"))
    run_stage("glass-relay", lambda: run_relay(args, results / "glass-relay"))
    run_stage("external-consumer", lambda: run_external_consumer(args, results / "external-consumer"))
    run_stage("pilot", lambda: run_pilot(args, results / "pilot", 1))
    run_stage("observation-matrix", lambda: run_pilot(args, results / "observation-matrix", 5))
    run_stage("semantic-equivalence", lambda: run_semantic_equivalence(args, results / "semantic-equivalence"))
    run_stage("memory-scaling", lambda: run_memory(args, results / "memory-scaling"))
    run_stage("rust-sink-cost", lambda: run_memory(args, results / "rust-sink-cost", sink_only=True))
    run_stage("python-sink-cost", lambda: run_python_sink(args, results / "python-sink-cost"))
    run_stage("heterogeneity-tables", lambda: run_tables())
    manifest["completed_at_utc"] = datetime.now(UTC).isoformat()
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


def run_tables() -> None:
    command(
        "render paper heterogeneity tables",
        [sys.executable, str(PAPER_ROOT / "generate_heterogeneity.py")],
        GLASSVM_ROOT,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "stage",
        choices=(
            "check",
            "all",
            "pilot",
            "observations",
            "memory",
            "rust-sink",
            "python-sink",
            "equivalence",
            "garden",
            "relay",
            "external-consumer",
            "tables",
        ),
    )
    parser.add_argument("--glassvm-root", type=Path, default=GLASSVM_ROOT)
    parser.add_argument("--machines-root", type=Path, default=DEFAULT_MACHINES_ROOT)
    parser.add_argument("--results-root", type=Path, default=DEFAULT_RESULTS_ROOT)
    args = parser.parse_args()
    args.glassvm_root = args.glassvm_root.resolve()
    args.machines_root = args.machines_root.resolve()
    args.results_root = args.results_root.resolve()

    try:
        if args.stage == "check":
            stage_check(args)
            return 0
        require_clean(args.glassvm_root, "GlassVM")
        require_clean(args.machines_root, "glassvm-machines")
        if args.stage == "tables":
            run_tables()
            return 0
        require_release_packages()
        output_by_stage = {
            "pilot": args.results_root / "pilot",
            "observations": args.results_root / "observation-matrix",
            "memory": args.results_root / "memory-scaling",
            "rust-sink": args.results_root / "rust-sink-cost",
            "python-sink": args.results_root / "python-sink-cost",
            "equivalence": args.results_root / "semantic-equivalence",
            "garden": args.results_root / "glass-garden",
            "relay": args.results_root / "glass-relay",
            "external-consumer": args.results_root / "external-consumer",
        }
        output = output_by_stage.get(args.stage)
        if output is not None:
            output.mkdir(parents=True, exist_ok=False)
        if args.stage == "all":
            run_all(args)
        elif args.stage == "pilot":
            run_pilot(args, output, 1)
        elif args.stage == "observations":
            run_pilot(args, output, 5)
        elif args.stage == "memory":
            run_memory(args, output)
        elif args.stage == "rust-sink":
            run_memory(args, output, sink_only=True)
        elif args.stage == "python-sink":
            run_python_sink(args, output)
        elif args.stage == "equivalence":
            run_semantic_equivalence(args, output)
        elif args.stage == "garden":
            run_garden(args, output)
        elif args.stage == "relay":
            run_relay(args, output)
        elif args.stage == "external-consumer":
            run_external_consumer(args, output)
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"paper runner failed: {error}", file=sys.stderr)
        return 1
    return 0


def clean_room_main(argv: list[str] | None = None) -> int:
    """Run the paper command in newly-created Python and Cargo environments."""
    argv = list(sys.argv[1:] if argv is None else argv)
    if os.environ.get("GLASSVM_PAPER_CLEAN_ROOM_CHILD") == "1":
        return main()
    if "--help" in argv or "-h" in argv:
        return main()

    uv = shutil.which("uv")
    if uv is None:
        print("paper runner requires uv on PATH to create its clean environment", file=sys.stderr)
        return 2

    try:
        with tempfile.TemporaryDirectory(prefix="glassvm-paper-clean-room-") as temporary:
            isolated = Path(temporary)
            python_env = isolated / "python-env"
            environment = os.environ.copy()
            for key in ("VIRTUAL_ENV", "PYTHONPATH", "PYTHONHOME"):
                environment.pop(key, None)
            environment.update(
                {
                    "GLASSVM_PAPER_CLEAN_ROOM_CHILD": "1",
                    "PYTHONNOUSERSITE": "1",
                    "UV_PROJECT_ENVIRONMENT": str(python_env),
                    "UV_CACHE_DIR": str(isolated / "uv-cache"),
                    "CARGO_HOME": str(isolated / "cargo-home"),
                    "CARGO_TARGET_DIR": str(isolated / "cargo-target"),
                }
            )
            print(
                "Creating disposable paper environment "
                f"(Python 3.12, fresh uv resolution, fresh crates.io cache, isolated build target): {isolated}",
                flush=True,
            )
            subprocess.run(
                [
                    uv,
                    "sync",
                    "--project",
                    str(PAPER_ROOT),
                    "--locked",
                    "--python",
                    "3.12",
                    "--no-cache",
                ],
                cwd=GLASSVM_ROOT,
                env=environment,
                check=True,
            )
            python = python_env / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
            return subprocess.run(
                [str(python), str(Path(__file__).resolve()), *argv],
                cwd=GLASSVM_ROOT,
                env=environment,
                check=False,
            ).returncode
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"failed to create clean paper environment: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(clean_room_main())
