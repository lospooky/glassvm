#!/usr/bin/env python3
"""Launch independent processes for the small Slice 4 accounting pilot."""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import tomllib

RECORDER_RECEIPT_SCHEMA = {
    "id": "glassvm.recorder_receipt",
    "version": {"major": 1, "minor": 1, "patch": 0},
}


def command_output(*command: str) -> str:
    try:
        return subprocess.check_output(command, text=True, stderr=subprocess.DEVNULL).strip()
    except (OSError, subprocess.CalledProcessError):
        return "unavailable"


def git_metadata(path: Path) -> dict[str, object]:
    revision = command_output("git", "-C", str(path), "rev-parse", "HEAD")
    status = command_output("git", "-C", str(path), "status", "--porcelain")
    return {
        "revision": None if revision == "unavailable" else revision,
        "dirty": status != "",
    }


def validate_result(result: dict, path: Path = Path("<result>")) -> dict:
    required = {
        "result_schema_version", "kind", "trial", "source", "host", "timing",
        "workload", "request", "negotiation", "outcomes", "evidence_channels",
        "recorder", "memory", "publication",
    }
    missing = required - result.keys()
    if missing:
        raise ValueError(f"{path} is missing required result fields: {sorted(missing)}")
    if result["result_schema_version"] != "1.0.0":
        raise ValueError(f"unsupported result schema in {path}")
    if result["kind"] == "run":
        if not {"execution_result", "evidence_receipt", "recorder_receipt"} <= result.keys():
            raise ValueError(f"{path} does not preserve all outcome receipts")
        receipt = result["recorder_receipt"]
        if receipt.get("schema") != RECORDER_RECEIPT_SCHEMA:
            raise ValueError(f"{path}: expected recorder receipt schema 1.1.0")
        channels = receipt.get("channels")
        if not isinstance(channels, list):
            raise ValueError(f"{path}: recorder receipt 1.1.0 lacks per-channel counters")
        aggregate_fields = ("segment_count", "record_count", "block_count", "logical_bytes")
        for field in aggregate_fields:
            channel_total = sum(item["stats"][field] for item in channels)
            if channel_total != receipt[field]:
                raise ValueError(f"{path}: per-channel {field} does not equal recorder aggregate")
        encoded_total = sum(item["stats"]["encoded_bytes"] for item in channels)
        if receipt["encoded_bytes"] is not None and encoded_total != receipt["encoded_bytes"]:
            raise ValueError(f"{path}: per-channel encoded bytes do not equal recorder aggregate")
        if result["publication"].get("published") and result["recorder"].get("status") != "complete":
            raise ValueError(f"{path}: published run lacks complete recorder status")
    elif result["kind"] == "excluded" and not result.get("exclusion_reason"):
        raise ValueError(f"{path}: exclusion is missing a reason")
    elif result["kind"] == "failed" and not result.get("failure", {}).get("message"):
        raise ValueError(f"{path}: failed trial is missing error details")
    elif result["kind"] not in {"excluded", "failed"}:
        raise ValueError(f"{path}: invalid result kind {result['kind']!r}")
    return result


def validate_trial(path: Path) -> dict:
    return validate_result(json.loads(path.read_text()), path)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--glassvm-root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--machines-root", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=1)
    parser.add_argument("--frame-limit", type=int, default=600)
    parser.add_argument("--machines", nargs="*", default=["chip8", "pico8", "tic80"])
    parser.add_argument("--recipes", nargs="*")
    args = parser.parse_args()
    if args.repetitions < 1:
        parser.error("--repetitions must be >= 1")
    paper_root = args.glassvm_root.resolve() / "docs/paper"
    recipe_data = tomllib.loads((paper_root / "experiments/OBSERVATION_RECIPES.toml").read_text())
    available_recipes = {recipe["id"] for recipe in recipe_data["recipe"]}
    if args.recipes is None:
        args.recipes = [recipe["id"] for recipe in recipe_data["recipe"]]
    unknown = set(args.recipes) - available_recipes
    if unknown:
        parser.error(f"unknown recipes: {sorted(unknown)}")
    args.output_dir.mkdir(parents=True, exist_ok=True)
    worker = Path(__file__).with_name("run_trial.py")
    uv_version = command_output("uv", "--version")
    rustc_version = command_output("rustc", "--version")
    source_snapshot = {
        "glassvm_checkout": git_metadata(args.glassvm_root.resolve()),
        "machines_checkout": git_metadata(args.machines_root.resolve()),
    }
    manifest = {
        "pilot_schema_version": "1.0.0",
        "measurement_policy": {
            "independent_process_per_trial": True,
            "repetitions": args.repetitions,
            "frame_limit": args.frame_limit,
            "host_os": platform.platform(),
            "cpu_architecture": platform.machine(),
            "uv": uv_version,
            "rustc": rustc_version,
            "allocation_high_water": "not exposed; reported null",
        },
        "trials": [],
    }
    for machine in args.machines:
        for recipe in args.recipes:
            for repetition in range(1, args.repetitions + 1):
                name = f"{machine}--{recipe}--r{repetition:02d}.json"
                result_path = args.output_dir / "raw" / name
                result_path.parent.mkdir(parents=True, exist_ok=True)
                command = [
                    sys.executable, "-I", str(worker),
                    "--paper-root", str(paper_root),
                    "--machines-root", str(args.machines_root.resolve()),
                    "--glassvm-root", str(args.glassvm_root.resolve()),
                    "--machine", machine,
                    "--recipe", recipe,
                    "--repetition", str(repetition),
                    "--frame-limit", str(args.frame_limit),
                    "--uv-version", uv_version,
                    "--rustc-version", rustc_version,
                    "--source-snapshot", json.dumps(source_snapshot, sort_keys=True),
                    "--output", str(result_path),
                ]
                start = time.perf_counter()
                completed = subprocess.run(command, text=True, capture_output=True)
                elapsed = time.perf_counter() - start
                if completed.returncode:
                    failure = {
                        "result_schema_version": "1.0.0",
                        "kind": "failed",
                        "trial": {"machine_id": machine, "recipe_id": recipe, "repetition": repetition, "started_utc": datetime.now(timezone.utc).isoformat()},
                        "source": {}, "host": {},
                        "timing": {"worker_process_seconds": elapsed},
                        "workload": {}, "request": {}, "negotiation": {}, "outcomes": {},
                        "evidence_channels": [], "recorder": {}, "memory": {},
                        "publication": {"published": False, "run_directory_bytes": None},
                        "failure": {
                            "phase": "trial_process",
                            "error_type": "WorkerExit",
                            "message": f"exit {completed.returncode}; stdout={completed.stdout!r}; stderr={completed.stderr!r}",
                        },
                    }
                    result_path.write_text(json.dumps(failure, indent=2, sort_keys=True) + "\n")
                result = validate_trial(result_path)
                result["timing"]["worker_process_seconds"] = elapsed
                result_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
                manifest["trials"].append({
                    "result": str(result_path.relative_to(args.output_dir)),
                    "machine_id": machine,
                    "recipe_id": recipe,
                    "repetition": repetition,
                    "kind": result["kind"],
                    "failure": result.get("failure"),
                    "recorder_status": result["recorder"].get("status"),
                    "recorder_channel_stats_available": result["recorder"].get("channel_stats_available"),
                    "evidence_status": result["outcomes"].get("evidence_status"),
                    "published": result["publication"].get("published"),
                })
                print(f"{machine}/{recipe}/r{repetition}: {result['kind']} in {elapsed:.3f}s")
    manifest_path = args.output_dir / "pilot_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(f"wrote {manifest_path}")


if __name__ == "__main__":
    main()
