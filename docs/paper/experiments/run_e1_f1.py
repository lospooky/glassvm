#!/usr/bin/env python3
"""Run E1 F1's unsupported required-capability case in fresh processes."""

import argparse
import hashlib
import importlib.metadata
import importlib.util
import json
import platform
import subprocess
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path

MACHINES = Path(__file__).resolve().parents[4] / "glassvm-machines"
HELPERS = MACHINES / "conformance/paper/ce1/run_ce1.py"
ID = "glassvm.paper.unsupported-capability"
EXPECTED = {"glassvm": "0.3.2", **{f"glassvm-{m}": "0.3.3" for m in ("chip8", "pico8", "tic80")}}


def worker(machine, repetition):
    spec = importlib.util.spec_from_file_location("ce1_helpers", HELPERS)
    helpers = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helpers)
    base = MACHINES / machine / "fixtures/paper"
    if machine != "chip8":
        base /= "ce1"
    cohort = tomllib.loads((base / "cohort.toml").read_text())
    workload = cohort["workload"][0]
    artifact = (MACHINES / machine / "fixtures" / workload["artifact_path"]).read_bytes()
    digest = hashlib.sha256(artifact).hexdigest()
    if digest != workload["sha256"]:
        raise ValueError("fixture digest mismatch")
    request = helpers.observation_request({"normalized_events": "none", "frames": "none"}, cohort)
    request["capabilities"] = [{"id": ID, "required": True, "parameters": {}}]
    config = helpers.machine_configuration(machine, cohort["configuration"])
    schedule = {"schema": helpers.INPUT_SCHEDULE_SCHEMA, "entries": []}
    controls = {"frame_limit": 600, "step_limit": None, "bundle_limits": []}
    import glassvm
    runtime = glassvm.Runtime.discover()
    row = {"case": "F1-required-capability", "machine": machine, "repetition": repetition,
           "artifact_path": workload["artifact_path"], "artifact_sha256": digest,
           "configuration": config, "input_schedule": schedule, "execution_controls": controls,
           "observation": request, "started_utc": datetime.now(timezone.utc).isoformat(),
           "preparation": None, "machine_execution": "not_started", "evidence_fulfillment": "not_started",
           "consumer_delivery": "not_requested", "recorder_finalization": "not_started",
           "publication": "not_attempted", "execution_invoked": False}
    try:
        runtime.prepare(machine, artifact, configuration=config, input_schedule=schedule,
                        execution_controls=controls, observation=request)
        row["preparation"] = {"status": "unexpected_success"}
    except Exception as error:
        row["preparation"] = {"status": "rejected", "error_type": type(error).__name__, "message": str(error)}
    return row


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--worker", choices=("chip8", "pico8", "tic80"))
    parser.add_argument("--repetition", type=int)
    args = parser.parse_args()
    if args.worker:
        print(json.dumps(worker(args.worker, args.repetition), sort_keys=True))
        return
    if not args.output_dir or (args.output_dir.exists() and any(args.output_dir.iterdir())):
        parser.error("output directory must be provided and empty")
    versions = {name: importlib.metadata.version(name) for name in EXPECTED}
    if versions != EXPECTED:
        raise SystemExit(f"unexpected installed packages: {versions}")
    args.output_dir.mkdir(parents=True, exist_ok=True)
    rows = []
    for machine in ("chip8", "pico8", "tic80"):
        for repetition in range(1, 4):
            cmd = [sys.executable, "-I", str(Path(__file__).resolve()), "--worker", machine, "--repetition", str(repetition)]
            result = subprocess.run(cmd, capture_output=True, text=True)
            path = args.output_dir / f"{machine}-r{repetition}.json"
            if result.returncode == 0:
                row = json.loads(result.stdout)
            else:
                row = {"case": "F1-required-capability", "machine": machine, "repetition": repetition,
                       "worker_error": {"returncode": result.returncode, "stdout": result.stdout, "stderr": result.stderr}}
            path.write_text(json.dumps(row, indent=2, sort_keys=True) + "\n")
            rows.append(row)
            print(f"{machine} repetition {repetition}: {row.get('preparation', row.get('worker_error'))}", flush=True)
    manifest = {"case": "F1-required-capability", "packages": versions, "python": sys.version,
                "host": platform.platform(), "machines_revision": subprocess.check_output(["git", "-C", str(MACHINES), "rev-parse", "HEAD"], text=True).strip(),
                "glassvm_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                "trials": len(rows), "rows": rows}
    (args.output_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    if any(row.get("preparation", {}).get("status") != "rejected" or ID not in row["preparation"].get("message", "") for row in rows):
        raise SystemExit("one or more trials did not reject the requested capability")


if __name__ == "__main__":
    main()
