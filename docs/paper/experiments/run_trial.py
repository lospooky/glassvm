#!/usr/bin/env python3
"""Execute one isolated paper trial through the installed GlassVM Python API."""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import os
import platform
import resource
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

import tomllib


RESULT_SCHEMA_VERSION = "1.0.0"
OBSERVATION_SCHEMA_VERSION = {"major": 2, "minor": 1, "patch": 0}
INPUT_SCHEDULE_SCHEMA = {
    "id": "glassvm.input_schedule",
    "version": {"major": 1, "minor": 0, "patch": 0},
}
CONFIGURATION_SCHEMAS = {
    "chip8": "chip8.machine_configuration",
    "pico8": "pico8.machine_configuration",
    "tic80": "tic80.machine_configuration",
}
NATIVE_EXECUTION_SUMMARY = {
    "chip8": "chip8.execution_summary",
    "pico8": "pico8.execution_summary",
    "tic80": "tic80.execution_summary",
}


def git_metadata(path: Path) -> dict[str, object]:
    def git(*args: str) -> str | None:
        try:
            return subprocess.check_output(
                ["git", "-C", str(path), *args], text=True, stderr=subprocess.DEVNULL
            ).strip()
        except (OSError, subprocess.CalledProcessError):
            return None

    return {"revision": git("rev-parse", "HEAD"), "dirty": git("status", "--porcelain") != ""}


def package_version(name: str) -> str | None:
    try:
        return importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        return None


def typed_value(value: object) -> dict[str, object]:
    if value is None:
        return {"Null": None}
    if isinstance(value, bool):
        return {"Bool": value}
    if isinstance(value, int):
        return {"Unsigned": value} if value >= 0 else {"Signed": value}
    if isinstance(value, float):
        return {"Float": value}
    if isinstance(value, str):
        return {"Text": value}
    if isinstance(value, list):
        return {"Array": [typed_value(item) for item in value]}
    if isinstance(value, dict):
        return {"Map": {key: typed_value(item) for key, item in value.items()}}
    raise TypeError(f"unsupported workload value type: {type(value).__name__}")


def event_kind(value: str) -> object:
    if value.startswith("extension:"):
        return {"extension": value.removeprefix("extension:")}
    return value


def observation_request(recipe: dict, workload: dict) -> dict:
    normalized = recipe["normalized"]
    if normalized == "none":
        events: object = "none"
    elif normalized == "lifecycle":
        events = "lifecycle"
    elif normalized == "all":
        events = "all"
    elif normalized == "workload":
        events = {"kinds": [event_kind(item) for item in workload["normalized_event_kinds"]]}
    else:
        raise ValueError(f"unknown normalized-event recipe selector {normalized!r}")

    native_mode = recipe["native"]
    native_enabled = native_mode != "none"
    native_kinds = (
        [NATIVE_EXECUTION_SUMMARY[workload["machine_id"]]]
        if native_mode == "execution_summary"
        else []
    )
    capabilities = []
    if recipe["capability"]:
        capabilities.append({"id": workload["capability"], "required": True, "parameters": {}})

    return {
        "schema_version": OBSERVATION_SCHEMA_VERSION,
        "guarantee": "summary",
        "normalized_events": {
            "events": events,
            "access_detail": "none",
            "state_diffs": False,
        },
        "capabilities": capabilities,
        "native_evidence": {"enabled": native_enabled, "all": False, "kinds": native_kinds},
        "frames": {"capture": recipe["frames"]},
        "snapshots": {"capture": "none"},
        "input_value_evidence": {"enabled": False},
        "budgets": {},
        "overflow": "fail",
    }


def machine_configuration(machine_id: str, values: dict) -> dict:
    return {
        "schema": {
            "id": CONFIGURATION_SCHEMAS[machine_id],
            "version": {"major": 1 if machine_id != "chip8" else 2, "minor": 0, "patch": 0},
        },
        "value": {"Map": {key: typed_value(value) for key, value in values.items()}},
    }


def directory_bytes(path: Path) -> int:
    total = 0
    pending = [path]
    while pending:
        current = pending.pop()
        with os.scandir(current) as entries:
            for entry in entries:
                if entry.is_dir(follow_symlinks=False):
                    pending.append(Path(entry.path))
                elif entry.is_file(follow_symlinks=False):
                    total += entry.stat(follow_symlinks=False).st_size
    return total


def bytes_from_artifact(path: Path) -> tuple[bytes, str]:
    artifact = path.read_bytes()
    return artifact, hashlib.sha256(artifact).hexdigest()


def run_trial(args: argparse.Namespace) -> dict:
    paper_root = Path(args.paper_root).resolve()
    machines_root = Path(args.machines_root).resolve()
    workload_data = tomllib.loads((paper_root / "PAPER_WORKLOADS.toml").read_text())
    recipes_data = tomllib.loads((paper_root / "experiments/OBSERVATION_RECIPES.toml").read_text())
    workload = next((row for row in workload_data["workload"] if row["machine_id"] == args.machine), None)
    recipe = next((row for row in recipes_data["recipe"] if row["id"] == args.recipe), None)
    if workload is None or recipe is None:
        raise ValueError("unknown machine workload or observation recipe")
    if args.machine not in recipes_data["machines"]:
        raise ValueError(f"{args.machine} is not in the explicit recipe support set")

    artifact_path = machines_root / workload["artifact_path"]
    artifact, artifact_digest = bytes_from_artifact(artifact_path)
    if artifact_digest != workload["artifact_sha256"]:
        raise ValueError(f"fixture hash mismatch for {artifact_path}: {artifact_digest}")

    started = datetime.now(timezone.utc).isoformat()
    process_start = time.perf_counter()
    rss_before = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    import glassvm

    discovery_start = time.perf_counter()
    runtime = glassvm.Runtime.discover()
    providers = json.loads(runtime.bundles())
    discovery_seconds = time.perf_counter() - discovery_start
    provider = next((item for item in providers if item["machine_id"] == args.machine), None)
    base = {
        "result_schema_version": RESULT_SCHEMA_VERSION,
        "kind": "run" if provider else "excluded",
        "trial": {
            "machine_id": args.machine,
            "recipe_id": args.recipe,
            "repetition": args.repetition,
            "started_utc": started,
        },
        "source": {
            "workspace_references": {
                "glassvm_checkout": git_metadata(Path(args.glassvm_root)),
                "machines_checkout": git_metadata(machines_root),
            },
            "installed_distribution_source_revision": None,
            "installed_distribution_source_note": "Python provider metadata does not embed a source revision; distribution versions and provider metadata are recorded separately.",
            "provider": provider,
        },
        "host": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "processor": platform.processor() or None,
            "python": sys.version,
            "python_implementation": platform.python_implementation(),
            "uv_version": args.uv_version,
            "packages": {
                name: package_version(name)
                for name in ("glassvm", f"glassvm-{args.machine}")
            },
            "rustc": args.rustc_version,
        },
        "timing": {"discovery_seconds": discovery_seconds},
        "workload": {
            "artifact_sha256": artifact_digest,
            "artifact_bytes": len(artifact),
            "artifact_path": workload["artifact_path"],
            "configuration": workload["configuration"],
            "input_schedule": "empty",
            "execution_controls": {"frame_limit": args.frame_limit, "step_limit": None, "bundle_limits": []},
        },
        "request": {},
        "negotiation": {
            "prepared_run_identity": None,
            "prepared_run_identity_note": "Opaque Python PreparedRun exposes no identity accessor; no surrogate derived from caller syntax is reported.",
            "prepared_observation_identity": None,
            "optional_unavailable_capabilities": [],
        },
        "outcomes": {},
        "evidence_channels": [],
        "recorder": {},
        "memory": {},
        "publication": {"published": False, "run_directory_bytes": None},
    }
    if provider is None:
        base["exclusion_reason"] = "No installed entry-point provider for this machine."
        base["kind"] = "excluded"
        base["timing"].update({"preparation_seconds": None, "execute_and_publish_seconds": None})
        base["memory"] = {"peak_rss_bytes": None, "allocator_high_water_bytes": None, "allocator_note": "No portable allocator high-water API is exposed."}
        return base

    request_observation = observation_request(recipe, workload)
    input_schedule = {"schema": INPUT_SCHEDULE_SCHEMA, "entries": []}
    execution_controls = {"frame_limit": args.frame_limit, "step_limit": None, "bundle_limits": []}
    config = machine_configuration(args.machine, workload["configuration"])
    base["request"] = {
        "configuration": config,
        "input_schedule": input_schedule,
        "execution_controls": execution_controls,
        "observation": request_observation,
    }
    prepare_start = time.perf_counter()
    try:
        prepared = runtime.prepare(
            args.machine,
            artifact,
            configuration=config,
            input_schedule=input_schedule,
            execution_controls=execution_controls,
            observation=request_observation,
        )
    except Exception as error:
        base["kind"] = "failed"
        base["failure"] = {"phase": "preparation", "error_type": type(error).__name__, "message": str(error)}
        base["timing"].update({"preparation_seconds": time.perf_counter() - prepare_start, "execute_and_publish_seconds": None})
        base["outcomes"] = {"machine_execution": "not_started", "evidence_status": "not_started", "recorder_status": "not_started"}
        base["memory"] = {"peak_rss_bytes": None, "allocator_high_water_bytes": None, "allocator_note": "No portable allocator high-water API is exposed."}
        return base
    preparation_seconds = time.perf_counter() - prepare_start

    with tempfile.TemporaryDirectory(prefix=f"glassvm-paper-{args.machine}-") as temporary:
        run_path = Path(temporary) / "published-run"
        execute_start = time.perf_counter()
        try:
            result = prepared.execute(str(run_path))
        except Exception as error:
            base["kind"] = "failed"
            base["failure"] = {"phase": "execute_and_publish", "error_type": type(error).__name__, "message": str(error)}
            base["timing"].update({
                "preparation_seconds": preparation_seconds,
                "execute_and_publish_seconds": time.perf_counter() - execute_start,
                "process_lifecycle_seconds": time.perf_counter() - process_start,
            })
            base["outcomes"] = {"machine_execution": "unknown", "evidence_status": "unknown", "recorder_status": "unknown"}
            base["memory"] = {"peak_rss_bytes": None, "allocator_high_water_bytes": None, "allocator_note": "Trial terminated before measurements were complete."}
            return base
        execute_publish_seconds = time.perf_counter() - execute_start
        if not isinstance(result, dict):
            raise TypeError("PreparedRun.execute must return a structured result mapping")
        execution = result.get("execution")
        evidence = result.get("evidence_receipt")
        recorder = result.get("recorder_receipt")
        published = result.get("published_run")
        if not all(isinstance(part, dict) for part in (execution, evidence, recorder, published)):
            raise ValueError("provider result does not expose all four structured outcome domains")

        manifest = json.loads((run_path / "manifest.json").read_text()) if (run_path / "manifest.json").exists() else {}
        prepared_id = manifest.get("prepared_observation_id")
        capability_receipts = evidence.get("capabilities", [])
        base["negotiation"].update({
            "prepared_observation_identity": prepared_id,
            "optional_unavailable_capabilities": [
                item for item in capability_receipts
                if item.get("status") == "unavailable"
            ],
        })
        base["outcomes"] = {
            "machine_execution": execution.get("common"),
            "evidence_status": evidence.get("status"),
            "recorder_status": recorder.get("status"),
        }
        base["execution_result"] = execution
        base["evidence_receipt"] = evidence
        base["recorder_receipt"] = recorder
        base["evidence_channels"] = evidence.get("channels", [])
        base["recorder"] = {
            "schema": recorder.get("schema"),
            "status": recorder.get("status"),
            "finalized": recorder.get("finalized"),
            "channel_stats_available": isinstance(recorder.get("channels"), list),
            "channel_stats_note": (
                None if isinstance(recorder.get("channels"), list)
                else "Installed recorder receipt predates per-channel bounded counters."
            ),
            "record_count": recorder.get("record_count"),
            "logical_bytes": recorder.get("logical_bytes"),
            "encoded_bytes": recorder.get("encoded_bytes"),
            "block_count": recorder.get("block_count"),
            "segment_count": recorder.get("segment_count"),
            "file_count": recorder.get("file_count"),
            "channels": recorder.get("channels"),
        }
        base["publication"] = {
            "published": bool(published.get("published")),
            "run_directory_bytes": directory_bytes(run_path) if run_path.exists() else None,
            "run_reference_retained": False,
        }
        base["timing"].update({
            "preparation_seconds": preparation_seconds,
            "execute_and_publish_seconds": execute_publish_seconds,
            "process_lifecycle_seconds": time.perf_counter() - process_start,
        })

    # ru_maxrss is KiB on Linux; byte conversion is recorded explicitly.
    peak_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    base["memory"] = {
        "peak_rss_bytes": int(peak_rss * 1024) if platform.system() == "Linux" else None,
        "peak_rss_native_value": peak_rss,
        "peak_rss_native_unit": "KiB" if platform.system() == "Linux" else "platform-specific",
        "allocator_high_water_bytes": None,
        "allocator_note": "No allocator high-water API is exposed by the Python provider protocol.",
        "rss_before_native_value": rss_before,
    }
    return base


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--paper-root", required=True)
    parser.add_argument("--machines-root", required=True)
    parser.add_argument("--glassvm-root", required=True)
    parser.add_argument("--machine", required=True)
    parser.add_argument("--recipe", required=True)
    parser.add_argument("--repetition", type=int, required=True)
    parser.add_argument("--frame-limit", type=int, default=600)
    parser.add_argument("--uv-version", default="unknown")
    parser.add_argument("--rustc-version", default="unknown")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    result = run_trial(args)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"result": str(output), "kind": result["kind"]}))


if __name__ == "__main__":
    main()
