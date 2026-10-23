#!/usr/bin/env python3
"""Validate and summarize the Rust sink-cost decomposition trials."""

from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from pathlib import Path

from summarize_memory_experiment import load_trials, validate_trials


EXPECTED_MODES = {"disabled", "normalizer_null", "recorder"}


def summarize(root: Path) -> dict:
    metadata = json.loads((root / "experiment.json").read_text())
    if set(metadata["modes"]) != EXPECTED_MODES:
        raise ValueError(f"expected exactly these modes: {sorted(EXPECTED_MODES)}")
    if len(metadata["frame_limits"]) != 1 or metadata["frame_limits"][0] != 60_000:
        raise ValueError("sink-cost trials must use the fixed 60,000-frame workload")

    trials = load_trials(root)
    validate_trials(trials, metadata)
    if any(trial.get("schema_version") != "1.1.0" for trial in trials):
        raise ValueError("sink-cost trials must use paper_memory_trial schema 1.1.0")

    grouped: dict[tuple[str, str], list[dict]] = defaultdict(list)
    for trial in trials:
        item = trial["trial"]
        grouped[(item["machine_id"], item["mode"])].append(trial)

    machines = metadata["machines"]
    repetitions = metadata["repetitions"]["60000"]
    rows = {}
    for machine in machines:
        per_mode = {mode: grouped[(machine, mode)] for mode in EXPECTED_MODES}
        if any(len(group) != repetitions for group in per_mode.values()):
            raise ValueError(f"{machine}: incomplete repetition groups")

        indexed = {
            mode: {trial["trial"]["repetition"]: trial for trial in group}
            for mode, group in per_mode.items()
        }
        for repetition in range(1, repetitions + 1):
            same_repetition = {
                mode: indexed[mode][repetition] for mode in EXPECTED_MODES
            }
            null_trial = same_repetition["normalizer_null"]
            recorder_trial = same_repetition["recorder"]
            reference = same_repetition["disabled"]
            for mode, trial in same_repetition.items():
                for field in ("configuration", "input_schedule", "execution_controls"):
                    if trial["request"][field] != reference["request"][field]:
                        raise ValueError(
                            f"{machine} repetition {repetition}: {field} varies by mode"
                        )
                if trial["source"]["artifact_sha256"] != reference["source"]["artifact_sha256"]:
                    raise ValueError(
                        f"{machine} repetition {repetition}: artifact varies by mode"
                    )
            if null_trial["request"] != recorder_trial["request"]:
                raise ValueError(
                    f"{machine} repetition {repetition}: NullSink and recorder requests differ"
                )
            if null_trial["evidence_receipt"] != recorder_trial["evidence_receipt"]:
                raise ValueError(
                    f"{machine} repetition {repetition}: NullSink and recorder evidence receipts differ"
                )
            run_ids = {
                indexed[mode][repetition]["evidence_receipt"]["run_id"]
                for mode in EXPECTED_MODES
            }
            if len(run_ids) != 1:
                raise ValueError(
                    f"{machine} repetition {repetition}: run identity varies by mode"
                )

        lifecycle = {
            mode: [trial["timing"]["execution_lifecycle_seconds"] for trial in group]
            for mode, group in per_mode.items()
        }
        medians = {mode: statistics.median(values) for mode, values in lifecycle.items()}
        if medians["disabled"] <= 0 or medians["normalizer_null"] <= 0:
            raise ValueError(f"{machine}: non-positive lifecycle time prevents overhead comparison")

        def relative_overhead(numerator: str, denominator: str) -> float:
            return (medians[numerator] / medians[denominator] - 1.0) * 100.0

        recorder_trials = per_mode["recorder"]
        rows[machine] = {
            "repetitions": repetitions,
            "frame_limit": 60_000,
            "lifecycle_seconds": {
                mode: {
                    "median": medians[mode],
                    "min": min(values),
                    "max": max(values),
                }
                for mode, values in lifecycle.items()
            },
            "process_wall_seconds_median": {
                mode: statistics.median(
                    trial["measurement"]["process_wall_seconds"]
                    for trial in group
                )
                for mode, group in per_mode.items()
            },
            "relative_overhead_percent": {
                "observation_pipeline_vs_disabled": relative_overhead(
                    "normalizer_null", "disabled"
                ),
                "recorder_vs_same_request_null_sink": relative_overhead(
                    "recorder", "normalizer_null"
                ),
                "recorder_vs_disabled": relative_overhead("recorder", "disabled"),
            },
            "absolute_overhead_seconds": {
                "observation_pipeline_vs_disabled": medians["normalizer_null"]
                - medians["disabled"],
                "recorder_vs_same_request_null_sink": medians["recorder"]
                - medians["normalizer_null"],
                "recorder_vs_disabled": medians["recorder"] - medians["disabled"],
            },
            "evidence_logical_bytes_median": statistics.median(
                trial["evidence_receipt"]["budgets"]["observed_logical_bytes"]
                for trial in recorder_trials
            ),
            "recorder_encoded_bytes_median": statistics.median(
                trial["recorder_receipt"]["encoded_bytes"]
                for trial in recorder_trials
            ),
            "directory_bytes_median": statistics.median(
                trial["publication"]["directory_bytes"]
                for trial in recorder_trials
            ),
            "directory_measurement_seconds_median": statistics.median(
                trial["timing"]["directory_measurement_seconds"]
                for trial in recorder_trials
            ),
        }
    return {
        "schema": "glassvm.paper_sink_cost_summary",
        "schema_version": "1.0.0",
        "source": metadata["source"],
        "source_is_clean": all(
            not metadata["source"][name]["dirty"] for name in ("glassvm", "machines")
        ),
        "machines": rows,
    }


def render_markdown(summary: dict) -> str:
    lines = [
        "# Rust sink-cost decomposition",
        "",
        "Three repetitions at a fixed 60,000-frame workload per machine.",
        "Lifecycle time includes machine execution plus the selected sink path; the recorder row includes finalization and publication.",
        "The NullSink and recorder rows use identical configuration, schedule, controls, and observation request.",
        "Process wall time is reported separately and includes process startup, preparation, and post-publication directory-size measurement.",
        f"Source checkouts recorded clean: {'yes' if summary['source_is_clean'] else 'no (exploratory; regenerate from clean pinned revisions before paper use)'}.",
        "",
        "| Machine | Disabled (s) | Observation + NullSink (s) | Recorder (s) | Observation delta (s, %) | Recorder delta vs NullSink (s, %) | Recorder delta vs disabled (s, %) | Evidence logical (MiB) | Recorder encoded (MiB) |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for machine, row in summary["machines"].items():
        lifecycle = row["lifecycle_seconds"]
        overhead = row["relative_overhead_percent"]
        delta = row["absolute_overhead_seconds"]
        lines.append(
            f"| {machine} | {lifecycle['disabled']['median']:.3f} | "
            f"{lifecycle['normalizer_null']['median']:.3f} | "
            f"{lifecycle['recorder']['median']:.3f} | "
            f"{delta['observation_pipeline_vs_disabled']:+.3f}, {overhead['observation_pipeline_vs_disabled']:+.1f}% | "
            f"{delta['recorder_vs_same_request_null_sink']:+.3f}, {overhead['recorder_vs_same_request_null_sink']:+.1f}% | "
            f"{delta['recorder_vs_disabled']:+.3f}, {overhead['recorder_vs_disabled']:+.1f}% | "
            f"{row['evidence_logical_bytes_median'] / (1024 * 1024):.2f} | "
            f"{row['recorder_encoded_bytes_median'] / (1024 * 1024):.2f} |"
        )
    lines.extend(
        (
            "",
            "Process-wall medians (seconds; includes startup, preparation, and post-publication directory-size measurement):",
            "",
            "| Machine | Disabled | Observation + NullSink | Recorder |",
            "|---|---:|---:|---:|",
        )
    )
    for machine, row in summary["machines"].items():
        wall = row["process_wall_seconds_median"]
        lines.append(
            f"| {machine} | {wall['disabled']:.2f} | "
            f"{wall['normalizer_null']:.2f} | {wall['recorder']:.2f} |"
        )
    lines.extend(
        (
            "",
            "These are workload- and host-specific medians, not universal overhead factors. "
            "The disabled-to-NullSink contrast includes event/frame production and online normalization, "
            "while the NullSink-to-recorder contrast estimates the incremental recorder lifecycle cost. "
            "Percentages with a very small disabled baseline should be read with their absolute deltas. "
            "Recorder encoded volume is storage output, not process memory.",
            "",
        )
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-dir", type=Path, required=True)
    args = parser.parse_args()
    root = args.data_dir.resolve()
    summary = summarize(root)
    (root / "sink-cost-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    (root / "SINK_COST_SUMMARY.md").write_text(render_markdown(summary))
    print(f"Validated sink-cost matrix; wrote summary in {root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
