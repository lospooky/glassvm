#!/usr/bin/env python3
"""Compare Python-consumer and Rust-recorder projections from shared runs.

The Python consumer streams caller-selected records to NDJSON. The reference
format is decoded only by the Rust recorder example; this script never parses
segment files. Records are materialized only afterward as an explicit finite
projection for comparison.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import tomllib
from pathlib import Path

from run_trial import machine_configuration, observation_request

MACHINES = ("chip8", "pico8", "tic80")
VERSION = "0.3.0"
CHANNEL_FOR_KIND = {
    "event": "normalized_events",
    "native_evidence": "native_evidence",
    "input_value_evidence": "input_value_evidence",
    "frame": "frames",
    "snapshot": "snapshots",
}
RECORDED_CHANNELS = (
    "normalized_events",
    "native_evidence",
    "input_value_evidence",
    "frames",
    "snapshots",
    "capability_outputs",
)


class ProjectionConsumer:
    """Write each callback record immediately; retain no run history."""

    def __init__(self, directory: Path) -> None:
        self.directory = directory
        self.files = {
            name: (directory / f"python-{name}.ndjson").open("w", encoding="utf-8")
            for name in (
                *RECORDED_CHANNELS,
                "capability_receipts",
                "lifecycle",
            )
        }
        self.counts = {name: 0 for name in self.files}

    def _write(self, name: str, value: object) -> None:
        self.files[name].write(json.dumps(value, sort_keys=True, separators=(",", ":")))
        self.files[name].write("\n")
        self.counts[name] += 1

    def emit_batch(self, emissions: list[dict]) -> None:
        for emission in emissions:
            kind = emission["kind"]
            record = emission["record"]
            channel = CHANNEL_FOR_KIND.get(kind)
            if channel:
                self._write(channel, record)
            elif kind in ("run_started", "run_finished", "evidence_receipt"):
                self._write("lifecycle", emission)
            else:
                raise ValueError(f"unrecognized Python emission kind: {kind!r}")

    def record_capability_outputs(self, outputs: list[dict]) -> None:
        for output in outputs:
            self._write("capability_outputs", output)

    def record_capability_receipts(self, receipts: list[dict]) -> None:
        for receipt in receipts:
            self._write("capability_receipts", receipt)

    def close(self) -> None:
        for file in self.files.values():
            file.close()


def read_projection(path: Path) -> list[object]:
    if not path.exists():
        return []
    with path.open(encoding="utf-8") as source:
        return [json.loads(line) for line in source if line.strip()]


def canonical(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def first_difference(left: list[object], right: list[object]) -> str | None:
    if len(left) != len(right):
        return f"record count differs: Python={len(left)}, Rust={len(right)}"
    for index, (python_record, rust_record) in enumerate(zip(left, right, strict=True)):
        if canonical(python_record) != canonical(rust_record):
            return (
                f"record {index} differs: Python={canonical(python_record)[:500]} "
                f"Rust={canonical(rust_record)[:500]}"
            )
    return None


def assert_equal(label: str, left: object, right: object) -> None:
    if canonical(left) != canonical(right):
        raise AssertionError(
            f"{label} differs:\nPython: {canonical(left)}\nRust:   {canonical(right)}"
        )


def run_machine(
    machine: str,
    workload: dict,
    recipe: dict,
    output_root: Path,
    glassvm_root: Path,
    machines_root: Path,
    frame_limit: int,
) -> dict:
    import glassvm

    directory = output_root / machine
    directory.mkdir(parents=True, exist_ok=True)
    artifact_path = machines_root / workload["artifact_path"]
    artifact = artifact_path.read_bytes()
    artifact_digest = hashlib.sha256(artifact).hexdigest()
    if artifact_digest != workload["artifact_sha256"]:
        raise ValueError(f"fixture hash mismatch for {artifact_path}: {artifact_digest}")

    configuration = machine_configuration(machine, workload["configuration"])
    schedule = {
        "schema": {
            "id": "glassvm.input_schedule",
            "version": {"major": 1, "minor": 0, "patch": 0},
        },
        "entries": [],
    }
    controls = {"frame_limit": frame_limit, "step_limit": None, "bundle_limits": []}
    observation = observation_request(recipe, workload)

    runtime = glassvm.Runtime.discover()
    provider = next(
        (entry for entry in json.loads(runtime.bundles()) if entry["machine_id"] == machine),
        None,
    )
    if provider is None:
        raise RuntimeError(f"no installed entry-point provider for {machine!r}")
    prepared = runtime.prepare(
        machine,
        artifact,
        configuration=configuration,
        input_schedule=schedule,
        execution_controls=controls,
        observation=observation,
    )

    consumer = ProjectionConsumer(directory)
    run_path = directory / "published-run"
    if run_path.exists():
        raise FileExistsError(
            f"{run_path} already exists; use a fresh --output directory for each run"
        )
    try:
        result = prepared.execute(
            sinks=[consumer],
            file_recorder=glassvm.FileRecorder(str(run_path)),
        )
    finally:
        consumer.close()
    if not isinstance(result, dict):
        raise TypeError("provider did not return a structured execution result")
    if result["python_sink_delivery"]["status"] != "complete":
        raise RuntimeError(f"Python evidence consumer failed: {result['python_sink_delivery']}")
    if result["publication"]["status"] != "published":
        raise RuntimeError(f"Rust reference recorder did not publish: {result['publication']}")

    rust_projection_path = directory / "rust-projection.json"
    subprocess.run(
        [
            "cargo",
            "run",
            "--locked",
            "--quiet",
            "-p",
            "glassvm_recorder",
            "--example",
            "project_reference_run",
            "--",
            str(run_path),
            str(rust_projection_path),
        ],
        cwd=glassvm_root,
        check=True,
    )
    rust = json.loads(rust_projection_path.read_text(encoding="utf-8"))

    python_channels = {
        name: read_projection(directory / f"python-{name}.ndjson")
        for name in RECORDED_CHANNELS
    }
    for channel in RECORDED_CHANNELS:
        difference = first_difference(python_channels[channel], rust["channels"][channel])
        if difference:
            raise AssertionError(f"{machine}/{channel}: {difference}")

    lifecycle = read_projection(directory / "python-lifecycle.ndjson")
    by_kind = {item["kind"]: item["record"] for item in lifecycle}
    if set(by_kind) != {"run_started", "evidence_receipt", "run_finished"}:
        raise AssertionError(f"{machine}: unexpected lifecycle callbacks: {set(by_kind)}")
    started = by_kind["run_started"]
    manifest = rust["manifest"]
    assert_equal("run ID", started["run_id"], manifest["run_id"])
    assert_equal("machine ID", started["machine_id"], manifest["machine_id"])
    assert_equal(
        "prepared observation identity",
        started["prepared_observation_id"],
        manifest["prepared_observation_id"],
    )
    assert_equal(
        "prepared run identity",
        started["prepared_run_identity"],
        manifest["prepared_run_identity"],
    )
    assert_equal("observation request", started["observation"], manifest["request"])
    assert_equal(
        "EvidenceReceipt emission",
        by_kind["evidence_receipt"],
        rust["evidence_receipt"],
    )
    assert_equal(
        "capability receipts",
        read_projection(directory / "python-capability_receipts.ndjson"),
        rust["evidence_receipt"]["capabilities"],
    )
    assert_equal(
        "recorder receipt in execution result",
        result["recorder_finalization"]["result"],
        rust["recorder_receipt"],
    )
    assert_equal(
        "RunFinished machine metrics",
        by_kind["run_finished"]["common"],
        result["machine_execution"]["result"]["common"],
    )

    # Compare against the equivalent explicit request, not caller dict order.
    assert_equal("prepared configuration request", started["configuration"], configuration)
    assert_equal("resolved input schedule request", started["input_schedule"], schedule)
    assert_equal("execution controls request", started["execution_controls"], controls)

    return {
        "machine_id": machine,
        "provider": provider,
        "artifact_sha256": artifact_digest,
        "frame_limit": frame_limit,
        "shared_execution": True,
        "prepared_observation_id": started["prepared_observation_id"],
        "prepared_run_identity": started["prepared_run_identity"],
        "equal_channels": {
            channel: len(python_channels[channel]) for channel in RECORDED_CHANNELS
        },
        "equal_capability_receipts": len(
            read_projection(directory / "python-capability_receipts.ndjson")
        ),
        "equal_evidence_receipt": True,
        "equal_machine_metrics": True,
        "execution_status": result["machine_execution"]["status"],
        "evidence_status": result["evidence_fulfillment"]["status"],
        "recorder_status": result["recorder_finalization"]["status"],
        "publication_status": result["publication"]["status"],
        "consumer_counts": consumer.counts,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--glassvm-root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--machines-root", type=Path, default=Path(__file__).resolve().parents[4] / "glassvm-machines")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--frames", type=int, default=600)
    args = parser.parse_args()

    glassvm_root = args.glassvm_root.resolve()
    machines_root = args.machines_root.resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    workload_data = tomllib.loads((glassvm_root / "docs/paper/PAPER_WORKLOADS.toml").read_text())
    recipes_data = tomllib.loads((glassvm_root / "docs/paper/experiments/OBSERVATION_RECIPES.toml").read_text())
    workloads = {row["machine_id"]: row for row in workload_data["workload"]}
    recipe = next(row for row in recipes_data["recipe"] if row["id"] == "mixed")
    results = [
        run_machine(
            machine,
            workloads[machine],
            recipe,
            output,
            glassvm_root,
            machines_root,
            args.frames,
        )
        for machine in MACHINES
    ]
    report = {
        "schema": "glassvm.paper_semantic_equivalence",
        "schema_version": "1.0.0",
        "glassvm_contract_crates": VERSION,
        "python_distributions": {
            "glassvm": VERSION,
            **{f"glassvm-{machine}": VERSION for machine in MACHINES},
        },
        "method": "one prepared execution fans out synchronously to Python consumer and Rust FileRecorder",
        "ordering": "compare only within each independent channel; no cross-channel merge order is inferred",
        "operational_metadata_excluded": [
            "segment/block bytes and boundaries",
            "recorder physical layout and filesystem paths",
            "wall-clock timing",
        ],
        "prepared_run_identity_comparison": "exact structured identity equality between Python RunStarted and Rust recorder manifest",
        "machines": results,
    }
    report_path = output / "equivalence-report.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(report_path)
    print(
        json.dumps(
            [
                {
                    "machine_id": row["machine_id"],
                    "equal_channels": row["equal_channels"],
                    "prepared_run_identity_match": True,
                    "prepared_observation_id": row["prepared_observation_id"],
                    "evidence_status": row["evidence_status"],
                    "publication_status": row["publication_status"],
                }
                for row in results
            ],
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
