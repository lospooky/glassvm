#!/usr/bin/env python3
"""Exercise published GlassVM providers without importing a GlassVM checkout."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import glassvm


MACHINES = {
    "chip8": {
        "artifact": "octojam2title.ch8",
        "schema": "chip8.machine_configuration",
        "version": 2,
        "configuration": {"quirks": "chip8", "cycles_per_frame": 12, "machine_seed": 0},
    },
    "pico8": {
        "artifact": "fireintro.p8",
        "schema": "pico8.machine_configuration",
        "version": 1,
        "configuration": {"cycles_per_frame": 1, "instruction_budget": 500000, "machine_seed": 0},
    },
    "tic80": {
        "artifact": "game.tic",
        "schema": "tic80.machine_configuration",
        "version": 1,
        "configuration": {"cycles_per_frame": 1, "runtime": "lua", "machine_seed": 0},
    },
}
INPUT_SCHEDULE = {
    "schema": {"id": "glassvm.input_schedule", "version": {"major": 1, "minor": 0, "patch": 0}},
    "entries": [],
}
OBSERVATION = {
    "schema_version": {"major": 2, "minor": 1, "patch": 0},
    "guarantee": "summary",
    "normalized_events": {"events": "all", "access_detail": "none", "state_diffs": False},
    "capabilities": [],
    "native_evidence": {"enabled": False, "all": False, "kinds": []},
    "frames": {"capture": "hashes"},
    "snapshots": {"capture": "none"},
    "input_value_evidence": {"enabled": False},
    "budgets": {},
    "overflow": "fail",
}


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
    raise TypeError(f"unsupported configuration value: {type(value).__name__}")


def machine_configuration(machine: str) -> dict[str, object]:
    spec = MACHINES[machine]
    return {
        "schema": {
            "id": spec["schema"],
            "version": {"major": spec["version"], "minor": 0, "patch": 0},
        },
        "value": {"Map": {key: typed_value(value) for key, value in spec["configuration"].items()}},
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--expect", nargs="*", default=[])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    expected = sorted(args.expect)

    runtime = glassvm.Runtime.discover()
    providers = json.loads(runtime.bundles())
    discovered = sorted(provider["machine_id"] for provider in providers)
    if discovered != expected:
        raise AssertionError(f"expected providers {expected}, discovered {discovered}")
    args.output.mkdir(parents=True, exist_ok=True)

    for machine in expected:
        spec = MACHINES[machine]
        artifact = (Path(__file__).parent / "fixtures" / spec["artifact"]).read_bytes()
        prepared = runtime.prepare(
            machine,
            artifact,
            configuration=machine_configuration(machine),
            input_schedule=INPUT_SCHEDULE,
            execution_controls={"frame_limit": 1, "step_limit": None, "bundle_limits": []},
            observation=OBSERVATION,
        )
        run_path = args.output / f"{machine}-published-run"
        result = prepared.execute(file_recorder=glassvm.FileRecorder(str(run_path)))
        required = {
            "machine_execution", "evidence_fulfillment", "recorder_finalization",
            "publication", "published_run", "python_sink_delivery",
        }
        if set(result) != required:
            raise AssertionError(f"unexpected structured result fields: {sorted(result)}")
        if result["machine_execution"]["status"] != "completed":
            raise AssertionError(f"machine execution did not complete for {machine}")
        if result["evidence_fulfillment"]["status"] != "complete":
            raise AssertionError(f"evidence not complete for {machine}: {result['evidence_fulfillment']}")
        if result["recorder_finalization"]["status"] != "finalized":
            raise AssertionError(f"recorder did not finalize for {machine}")
        if result["recorder_finalization"]["result"]["status"] != "complete":
            raise AssertionError(f"recorder receipt incomplete for {machine}")
        if (
            result["publication"]["status"] != "published"
            or result["published_run"] != {"path": str(run_path), "published": True}
            or not run_path.is_dir()
        ):
            raise AssertionError(f"run was not published for {machine}")
        print(
            f"{machine}: execution={result['machine_execution']['status']} "
            f"evidence={result['evidence_fulfillment']['status']} "
            f"recorder={result['recorder_finalization']['status']} "
            f"publication={result['publication']['status']}"
        )


if __name__ == "__main__":
    main()
