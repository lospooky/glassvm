#!/usr/bin/env python3
"""Repeat the E1 fault/accounting assertion suites in isolated Cargo runs."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import subprocess
import sys
import time
from pathlib import Path


GLASSVM_ROOT = Path(__file__).resolve().parents[3]
EXPECTED_VERSION = "0.3.2"
GLASSVM_COMMAND = [
    "cargo",
    "test",
    "--workspace",
    "--locked",
    "--no-fail-fast",
]
RECORDER_PYTHON_COMMAND = [
    "cargo",
    "test",
    "--package",
    "glassvm_recorder",
    "--features",
    "python",
    "--locked",
    "--no-fail-fast",
]
MACHINES_COMMAND = [
    "cargo",
    "test",
    "--workspace",
    "--locked",
    "--no-fail-fast",
]


def git_value(root: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=root, text=True).strip()


def package_version(path: Path, section: str) -> str:
    text = path.read_text(encoding="utf-8")
    section_name = "workspace.package" if section == "workspace" else section
    section_match = re.search(
        rf"(?ms)^\[{re.escape(section_name)}\]\s*(.*?)(?=^\[|\Z)", text
    )
    if section_match is None:
        raise SystemExit(f"{path} is missing [{section_name}]")
    version_match = re.search(r'(?m)^version\s*=\s*"([^"]+)"\s*$', section_match.group(1))
    if version_match is None:
        raise SystemExit(f"{path} is missing a literal version in [{section_name}]")
    return version_match.group(1)


def verify_versions(machines_root: Path) -> None:
    glassvm_version = package_version(GLASSVM_ROOT / "Cargo.toml", "workspace")
    if glassvm_version != EXPECTED_VERSION:
        raise SystemExit(f"GlassVM workspace is {glassvm_version}, expected {EXPECTED_VERSION}")
    for machine in ("chip8", "pico8", "tic80"):
        version = package_version(machines_root / machine / "pyproject.toml", "project")
        if version != EXPECTED_VERSION:
            raise SystemExit(f"{machine} distribution is {version}, expected {EXPECTED_VERSION}")
    selector_version = package_version(machines_root / "distribution/pyproject.toml", "project")
    if selector_version != EXPECTED_VERSION:
        raise SystemExit(f"machine selector is {selector_version}, expected {EXPECTED_VERSION}")


def run_stage(root: Path, command: list[str], log_path: Path) -> dict[str, object]:
    started = time.monotonic()
    completed = subprocess.run(
        command,
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=False,
    )
    elapsed = time.monotonic() - started
    log_path.write_text(completed.stdout, encoding="utf-8")
    summaries = [line.strip() for line in completed.stdout.splitlines() if "test result:" in line]
    return {
        "command": command,
        "exit_code": completed.returncode,
        "elapsed_seconds": round(elapsed, 3),
        "test_result_lines": summaries,
        "log": log_path.name,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--machines-root",
        type=Path,
        default=GLASSVM_ROOT.parent / "glassvm-machines",
    )
    parser.add_argument(
        "--results-dir",
        type=Path,
        default=GLASSVM_ROOT / "docs/paper/experiments/measurements/e1-repetitions-0.3.2",
    )
    args = parser.parse_args()
    machines_root = args.machines_root.resolve()
    results_dir = args.results_dir.resolve()
    verify_versions(machines_root)
    if results_dir.exists():
        raise SystemExit(f"Refusing to overwrite existing results: {results_dir}")
    results_dir.mkdir(parents=True)

    manifest = {
        "schema": "glassvm.e1-test-repetitions",
        "release": EXPECTED_VERSION,
        "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "repetitions": 3,
        "glassvm": {
            "root": str(GLASSVM_ROOT),
            "revision": git_value(GLASSVM_ROOT, "rev-parse", "HEAD"),
            "branch": git_value(GLASSVM_ROOT, "branch", "--show-current"),
        },
        "machines": {
            "root": str(machines_root),
            "revision": git_value(machines_root, "rev-parse", "HEAD"),
            "branch": git_value(machines_root, "branch", "--show-current"),
        },
        "scope": (
            "Repeated source-level fault/accounting and machine conformance test suites. "
            "These are not runtime benchmarks or new workload executions."
        ),
        "runs": [],
    }

    failed = False
    for repetition in range(1, 4):
        result: dict[str, object] = {"repetition": repetition, "stages": {}}
        stages = result["stages"]
        assert isinstance(stages, dict)
        stages["glassvm_workspace"] = run_stage(
            GLASSVM_ROOT,
            GLASSVM_COMMAND,
            results_dir / f"r{repetition:02}-glassvm-workspace.log",
        )
        stages["recorder_python"] = run_stage(
            GLASSVM_ROOT,
            RECORDER_PYTHON_COMMAND,
            results_dir / f"r{repetition:02}-recorder-python.log",
        )
        stages["machines"] = run_stage(
            machines_root,
            MACHINES_COMMAND,
            results_dir / f"r{repetition:02}-machines.log",
        )
        result["status"] = "pass" if all(
            stage["exit_code"] == 0 for stage in stages.values()
        ) else "fail"
        failed |= result["status"] != "pass"
        (results_dir / f"r{repetition:02}.json").write_text(
            json.dumps(result, indent=2) + "\n", encoding="utf-8"
        )
        manifest["runs"].append(result)
        (results_dir / "manifest.json").write_text(
            json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
        )
        print(
            f"repetition {repetition}: {result['status']} "
            "(GlassVM workspace "
            f"{stages['glassvm_workspace']['exit_code']}, recorder/Python "
            f"{stages['recorder_python']['exit_code']}, machines {stages['machines']['exit_code']})",
            flush=True,
        )
    manifest["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    (results_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
