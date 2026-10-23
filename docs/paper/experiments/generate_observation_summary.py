#!/usr/bin/env python3
"""Validate and summarize repeated selectable-observation measurements."""

from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from pathlib import Path

from run_pilot import validate_trial


def percentile_median(values: list[float | int | None]) -> str:
    present = [float(value) for value in values if value is not None]
    if not present:
        return "—"
    value = statistics.median(present)
    return f"{value:.6f}" if isinstance(values[0], float) else f"{int(value)}"


def channel_totals(result: dict, channel: str, source: str, field: str) -> int:
    records = result.get(source, [])
    return sum(
        int(item.get(field) or 0) for item in records if item.get("channel") == channel
    )


def collect_rows(root: Path) -> tuple[list[dict], list[str]]:
    manifest_path = root / "measurement_manifest.json"
    manifest = json.loads(manifest_path.read_text())
    expected_repetitions = manifest["measurement_policy"].get("repetitions")
    results = []
    for item in manifest["trials"]:
        result_path = root / item["result"]
        result = validate_trial(result_path)
        if (
            result["trial"]["machine_id"],
            result["trial"]["recipe_id"],
            result["trial"]["repetition"],
        ) != (item["machine_id"], item["recipe_id"], item["repetition"]):
            raise ValueError(f"manifest identity does not match {result_path}")
        results.append(result)

    groups: dict[tuple[str, str], list[dict]] = defaultdict(list)
    for result in results:
        groups[(result["trial"]["machine_id"], result["trial"]["recipe_id"])].append(
            result
        )

    anomalies = []
    machine_work: dict[str, set[tuple[object, object, object]]] = defaultdict(set)
    machine_inputs: dict[str, set[str]] = defaultdict(set)
    for result in results:
        if result["kind"] != "run":
            anomalies.append(
                f"{result['trial']['machine_id']}/{result['trial']['recipe_id']}/r{result['trial']['repetition']}: {result['kind']}"
            )
            continue
        execution = result["execution_result"].get("common", {})
        request = result.get("request", {})
        machine_inputs[result["trial"]["machine_id"]].add(
            json.dumps(
                {
                    "artifact_sha256": result.get("workload", {}).get(
                        "artifact_sha256"
                    ),
                    "configuration": request.get("configuration"),
                    "input_schedule": request.get("input_schedule"),
                    "execution_controls": request.get("execution_controls"),
                },
                sort_keys=True,
                separators=(",", ":"),
            )
        )
        machine_work[result["trial"]["machine_id"]].add(
            (
                execution.get("frames"),
                execution.get("cycles"),
                execution.get("termination"),
            )
        )
        if result["evidence_receipt"].get("status") != "complete":
            anomalies.append(
                f"{result['trial']['machine_id']}/{result['trial']['recipe_id']}/r{result['trial']['repetition']}: evidence not complete"
            )
        if result["recorder_receipt"].get("status") != "complete":
            anomalies.append(
                f"{result['trial']['machine_id']}/{result['trial']['recipe_id']}/r{result['trial']['repetition']}: recorder not complete"
            )
        if not result["publication"].get("published"):
            anomalies.append(
                f"{result['trial']['machine_id']}/{result['trial']['recipe_id']}/r{result['trial']['repetition']}: not published"
            )
        if result["evidence_receipt"].get("budgets", {}).get("exceeded") is not None:
            anomalies.append(
                f"{result['trial']['machine_id']}/{result['trial']['recipe_id']}/r{result['trial']['repetition']}: evidence budget exceeded"
            )
        configured_budgets = (
            result["evidence_receipt"].get("budgets", {}).get("configured", {})
        )
        if any(value is not None for value in configured_budgets.values()):
            anomalies.append(
                f"{result['trial']['machine_id']}/{result['trial']['recipe_id']}/r{result['trial']['repetition']}: configured evidence budget differs from the declared unbounded policy"
            )
        if result["recorder"].get("configured_limits") is None:
            anomalies.append(
                f"{result['trial']['machine_id']}/{result['trial']['recipe_id']}/r{result['trial']['repetition']}: recorder limits not recorded"
            )

    for machine, work in sorted(machine_work.items()):
        if len(work) > 1:
            anomalies.append(
                f"{machine}: execution work/termination differs across observation recipes or repetitions: {sorted(map(str, work))}"
            )
    for machine, materials in sorted(machine_inputs.items()):
        if len(materials) > 1:
            anomalies.append(
                f"{machine}: artifact/configuration/schedule/execution controls differ across observation recipes or repetitions"
            )

    lines = [
        "# Slice 5: selectable-observation measurements",
        "",
        "Each cell reports the median of independent process repetitions. Raw per-run",
        "observations remain authoritative in `raw/`; this table does not replace them.",
        "All recipes use identical artifact/configuration/empty-schedule material per",
        "machine and the same frame limit. Work is reported as the machine's returned",
        "frame/cycle coordinates and termination; cycle units are machine-specific.",
        "Evidence budgets are deliberately unbounded (all configured maxima are null).",
        "The Python providers configure hard recorder limits of 1 MiB per block, 256",
        "records per segment, 2 MiB per segment, and 1 MiB live buffer per channel.",
        "A measurement is complete only when execution returns, evidence and recorder",
        "receipts are complete, and publication succeeds. Any other outcome is retained",
        "as an anomaly rather than treated as an ordinary overhead observation.",
        "",
        "## Runtime, work, outcome, and total storage",
        "",
        "The recipe ID names the exact request in `OBSERVATION_RECIPES.toml`; its",
        "workload-specific resolved event/capability selectors are in the raw trial.",
        "Artifact keys are the first 12 hex digits of the fixture SHA-256; full",
        "digests and full GlassVM/machine source revisions remain in each raw trial.",
        "At collection, the GlassVM checkout was marked dirty for experiment/docs",
        "work and the machine checkout was clean; the exact flags are in every trial.",
        "",
        "| Machine | Recipe/request ID | Artifact key | Source commits GVM/machines | n | Frames / cycles / termination | Evidence / recorder / publish | Prepare s median | Execute+publish s median | Worker s median | Peak RSS bytes median | Evidence items / logical bytes | Recorder records / logical bytes / encoded bytes | Blocks / segments / files | Physical directory bytes |",
        "| --- | --- | --- | --- | ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for (machine, recipe), group in sorted(groups.items()):
        if expected_repetitions is not None and len(group) != expected_repetitions:
            anomalies.append(
                f"{machine}/{recipe}: expected {expected_repetitions} repetitions, found {len(group)}"
            )
        runs = [result for result in group if result["kind"] == "run"]
        if not runs:
            lines.append(
                f"| {machine} | {recipe} | 0 | — | failed/excluded | — | — | — | — | — | — | — | — |"
            )
            continue
        first = runs[0]
        common = first["execution_result"].get("common", {})
        artifact_key = first.get("workload", {}).get("artifact_sha256", "")[:12]
        refs = first.get("source", {}).get("workspace_references", {})
        glassvm_revision = (
            refs.get("glassvm_checkout", {}).get("revision") or "unknown"
        )[:8]
        machines_revision = (
            refs.get("machines_checkout", {}).get("revision") or "unknown"
        )[:8]
        evidence_items = sum(
            item.get("item_count", 0)
            for item in first.get("evidence_channels", [])
            if item.get("channel") != "envelope"
        )
        evidence_bytes = sum(
            item.get("logical_bytes", 0)
            for item in first.get("evidence_channels", [])
            if item.get("channel") != "envelope"
        )
        lines.append(
            "| {machine} | {recipe} | {artifact} | {glassvm}/{machines} | {n} | {frames} / {cycles} / {termination} | {evidence} / {recorder} / {published} | {prepare} | {execute} | {worker} | {rss} | {items} / {bytes} | {records} / {logical} / {encoded} | {blocks} / {segments} / {files} | {physical} |".format(
                machine=machine,
                recipe=recipe,
                artifact=artifact_key,
                glassvm=glassvm_revision,
                machines=machines_revision,
                n=len(runs),
                frames=common.get("frames", "—"),
                cycles=common.get("cycles", "—"),
                termination=common.get("termination", "—"),
                evidence=first["evidence_receipt"].get("status", "—"),
                recorder=first["recorder_receipt"].get("status", "—"),
                published=first["publication"].get("published", False),
                prepare=percentile_median(
                    [r["timing"].get("preparation_seconds") for r in runs]
                ),
                execute=percentile_median(
                    [r["timing"].get("execute_and_publish_seconds") for r in runs]
                ),
                worker=percentile_median(
                    [r["timing"].get("worker_process_seconds") for r in runs]
                ),
                rss=percentile_median(
                    [r["memory"].get("peak_rss_bytes") for r in runs]
                ),
                items=evidence_items,
                bytes=evidence_bytes,
                records=first["recorder"].get("record_count", "—"),
                logical=first["recorder"].get("logical_bytes", "—"),
                encoded=first["recorder"].get("encoded_bytes", "—"),
                blocks=first["recorder"].get("block_count", "—"),
                segments=first["recorder"].get("segment_count", "—"),
                files=first["recorder"].get("file_count", "—"),
                physical=first["publication"].get("run_directory_bytes", "—"),
            )
        )
        for result in runs[1:]:
            for channel in (
                "normalized_events",
                "native_evidence",
                "frames",
                "capabilities",
                "snapshots",
                "input_value_evidence",
            ):
                if channel_totals(
                    result, channel, "evidence_channels", "item_count"
                ) != channel_totals(
                    first, channel, "evidence_channels", "item_count"
                ) or channel_totals(
                    result, channel, "evidence_channels", "logical_bytes"
                ) != channel_totals(
                    first, channel, "evidence_channels", "logical_bytes"
                ):
                    anomalies.append(
                        f"{machine}/{recipe}: evidence channel {channel} differs across repetitions"
                    )
            if result["recorder_receipt"].get("channels") != first[
                "recorder_receipt"
            ].get("channels"):
                anomalies.append(
                    f"{machine}/{recipe}: per-channel recorder counters differ across repetitions"
                )
            for field in (
                "record_count",
                "logical_bytes",
                "encoded_bytes",
                "block_count",
                "segment_count",
                "file_count",
            ):
                if result["recorder"].get(field) != first["recorder"].get(field):
                    anomalies.append(
                        f"{machine}/{recipe}: recorder {field} differs across repetitions"
                    )
            if result["publication"].get("run_directory_bytes") != first[
                "publication"
            ].get("run_directory_bytes"):
                anomalies.append(
                    f"{machine}/{recipe}: physical directory bytes differ across repetitions"
                )

    lines.extend(
        [
            "",
            "## Per-channel accounting",
            "",
            "Values are `items/evidence-logical-bytes/recorder-encoded-bytes`; recorder encoded bytes are physical-channel accounting, not the evidence receipt's logical size.",
            "",
        ]
    )
    lines.append(
        "| Machine | Recipe | Normalized | Native | Frames | Capabilities | Snapshots | Input values |"
    )
    lines.append("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |")
    for key, group in sorted(groups.items()):
        result = next((item for item in group if item["kind"] == "run"), None)
        if result is None:
            continue
        vals = []
        for channel in (
            "normalized_events",
            "native_evidence",
            "frames",
            "capabilities",
            "snapshots",
            "input_value_evidence",
        ):
            items = channel_totals(result, channel, "evidence_channels", "item_count")
            logical = channel_totals(
                result, channel, "evidence_channels", "logical_bytes"
            )
            recorder_channel = (
                "capability_outputs" if channel == "capabilities" else channel
            )
            encoded = next(
                (
                    item.get("stats", {}).get("encoded_bytes")
                    for item in result["recorder_receipt"].get("channels", [])
                    if item.get("channel") == recorder_channel
                ),
                None,
            )
            vals.append(
                f"{items}/{logical}/{encoded if encoded is not None else 'n/a'}"
            )
        lines.append(f"| {key[0]} | {key[1]} | " + " | ".join(vals) + " |")

    lines.extend(["", "## Anomalies and interpretation", ""])
    if anomalies:
        lines.extend(f"- {item}" for item in sorted(set(anomalies)))
    else:
        lines.append("- None detected by the mechanical checks.")
    lines.extend(
        [
            "",
            "Runtime medians are descriptive for this host and these finite workloads;",
            "the isolated-process high-water RSS includes interpreter/provider startup,",
            "and is not an allocation-only or bounded-memory claim. Physical directory",
            "bytes include recorder framing and metadata; evidence logical bytes do not.",
            "",
        ]
    )
    return "\n".join(lines), sorted(set(anomalies))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--measurement-dir", type=Path, required=True)
    args = parser.parse_args()
    rendered, anomalies = collect_rows(args.measurement_dir)
    output = args.measurement_dir / "OBSERVATION_SUMMARY.md"
    output.write_text(rendered)
    print(f"wrote {output}; anomalies={len(anomalies)}")
    if anomalies:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
