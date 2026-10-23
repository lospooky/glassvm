#!/usr/bin/env python3
"""Validate and summarize the run-length memory/evidence experiment."""

from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from pathlib import Path

def load_trials(root: Path) -> list[dict]:
    trials = []
    for path in sorted(root.glob("*__*.json")):
        if path.name == "experiment.json":
            continue
        trial = json.loads(path.read_text())
        if trial.get("status") != "completed":
            raise ValueError(f"trial is not complete: {path.name} ({trial.get('status')})")
        trials.append(trial)
    return trials


def validate_trials(trials: list[dict], metadata: dict) -> None:
    expected = {
        (machine, mode, frames, repetition)
        for machine in metadata["machines"]
        for mode in metadata["modes"]
        for frames in metadata["frame_limits"]
        for repetition in range(1, metadata["repetitions"][str(frames)] + 1)
    }
    found = set()
    executions: dict[tuple[str, int], set[tuple[int, int, bool, str]]] = defaultdict(set)
    for trial in trials:
        item = trial["trial"]
        key = (item["machine_id"], item["mode"], item["frame_limit"], item["repetition"])
        if key in found:
            raise ValueError(f"duplicate trial identity: {key}")
        found.add(key)
        common = trial["execution"]["common"]
        if common["frames"] != item["frame_limit"]:
            raise ValueError(f"run did not reach its frame target: {key} -> {common['frames']}")
        executions[(item["machine_id"], item["frame_limit"])].add(
            (
                common["frames"],
                common["cycles"],
                common["boot_success"],
                common["termination"],
            )
        )
        if trial["evidence_receipt"]["status"] != "complete":
            raise ValueError(f"evidence fulfillment incomplete: {key}")
        if item["mode"] == "recorder":
            receipt = trial["recorder_receipt"]
            if receipt is None or receipt["status"] != "complete" or not receipt["finalized"]:
                raise ValueError(f"recorder did not finalize: {key}")
            if not trial["publication"]["published"]:
                raise ValueError(f"run was not atomically published: {key}")
        elif trial["recorder_receipt"] is not None or trial["publication"]["published"]:
            raise ValueError(f"non-recorder mode unexpectedly published: {key}")
    missing = expected - found
    unexpected = found - expected
    if missing or unexpected:
        raise ValueError(f"trial matrix mismatch; missing={sorted(missing)}, unexpected={sorted(unexpected)}")
    inconsistent = {key: values for key, values in executions.items() if len(values) != 1}
    if inconsistent:
        raise ValueError(f"execution diverged across observation modes/repetitions: {inconsistent}")


def metric_summary(trials: list[dict]) -> dict:
    groups: dict[tuple[str, str, int], list[dict]] = defaultdict(list)
    for trial in trials:
        item = trial["trial"]
        groups[(item["machine_id"], item["mode"], item["frame_limit"])].append(trial)
    result = {}
    for key, group in sorted(groups.items()):
        machine, mode, frames = key
        group.sort(key=lambda row: row["trial"]["repetition"])
        rss = [row["memory"]["peak_rss_bytes"] for row in group]
        wall = [row["measurement"]["process_wall_seconds"] for row in group]
        logical = [row["evidence_receipt"]["budgets"]["observed_logical_bytes"] for row in group]
        encoded = [
            row["recorder_receipt"]["encoded_bytes"] if row["recorder_receipt"] else None
            for row in group
        ]
        result[f"{machine}/{mode}/{frames}"] = {
            "machine_id": machine,
            "mode": mode,
            "frame_limit": frames,
            "repetitions": len(group),
            "peak_rss_bytes_median": statistics.median(rss),
            "peak_rss_bytes_min": min(rss),
            "peak_rss_bytes_max": max(rss),
            "process_wall_seconds_median": statistics.median(wall),
            "evidence_logical_bytes_median": statistics.median(logical),
            "recorder_encoded_bytes_median": statistics.median(
                [value for value in encoded if value is not None]
            ) if any(value is not None for value in encoded) else None,
        }
    return result


def write_plots(root: Path, rows: list[dict], machines: list[str], modes: list[str]) -> None:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    colors = {"chip8": "#2563eb", "pico8": "#d97706", "tic80": "#059669"}
    styles = {"disabled": "-", "events_null": "--", "normalizer_null": "-.", "recorder": ":"}
    markers = {"disabled": "o", "events_null": "s", "normalizer_null": "^", "recorder": "D"}

    fig, ax = plt.subplots(figsize=(10, 6))
    for machine in machines:
        for mode in modes:
            points = sorted(
                (row for row in rows if row["machine_id"] == machine and row["mode"] == mode),
                key=lambda row: row["frame_limit"],
            )
            ax.plot(
                [row["frame_limit"] for row in points],
                [row["peak_rss_bytes_median"] / (1024 * 1024) for row in points],
                color=colors[machine], linestyle=styles[mode], marker=markers[mode],
                label=f"{machine} / {mode}",
            )
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("Requested frames")
    ax.set_ylabel("Peak process RSS (MiB, median of repetitions)")
    ax.set_title("Peak memory vs run length")
    ax.grid(True, which="both", alpha=0.25)
    ax.legend(ncol=2, fontsize=8)
    fig.tight_layout()
    fig.savefig(root / "memory-vs-run-length.svg")
    plt.close(fig)

    fig, (logical_ax, encoded_ax) = plt.subplots(2, 1, figsize=(10, 9), sharex=True)
    for machine in machines:
        for mode in modes:
            points = sorted(
                (row for row in rows if row["machine_id"] == machine and row["mode"] == mode),
                key=lambda row: row["frame_limit"],
            )
            logical_ax.plot(
                [row["frame_limit"] for row in points],
                [row["evidence_logical_bytes_median"] / (1024 * 1024) for row in points],
                color=colors[machine], linestyle=styles[mode], marker=markers[mode],
                label=f"{machine} / {mode}",
            )
            encoded_points = [row for row in points if row["recorder_encoded_bytes_median"] is not None]
            if encoded_points:
                encoded_ax.plot(
                    [row["frame_limit"] for row in encoded_points],
                    [row["recorder_encoded_bytes_median"] / (1024 * 1024) for row in encoded_points],
                    color=colors[machine], linestyle=styles[mode], marker=markers[mode],
                    label=f"{machine} / {mode}",
                )
    for axis in (logical_ax, encoded_ax):
        axis.set_xscale("log")
        axis.set_yscale("symlog", linthresh=1)
        axis.grid(True, which="both", alpha=0.25)
        axis.legend(ncol=2, fontsize=8)
    logical_ax.set_ylabel("Evidence logical bytes (MiB)")
    logical_ax.set_title("Emitted evidence volume (EvidenceReceipt)")
    encoded_ax.set_ylabel("Encoded recorder bytes (MiB)")
    encoded_ax.set_xlabel("Requested frames")
    encoded_ax.set_title("Persisted bytes (RecorderReceipt)")
    fig.suptitle("Evidence volume vs run length")
    fig.tight_layout()
    fig.savefig(root / "evidence-volume-vs-run-length.svg")
    plt.close(fig)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-dir", type=Path, required=True)
    args = parser.parse_args()
    root = args.data_dir.resolve()
    metadata = json.loads((root / "experiment.json").read_text())
    trials = load_trials(root)
    validate_trials(trials, metadata)
    summary = metric_summary(trials)
    (root / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    machines = metadata["machines"]
    modes = metadata["modes"]
    write_plots(root, list(summary.values()), machines, modes)

    lines = [
        "# Run-length memory and evidence summary",
        "",
            "All trials reached the requested frame count. Observation modes produced identical frame/cycle totals, boot outcome, and termination within each machine and run length.",
            "Peak RSS is the per-process Linux `VmHWM`; three repetitions were used through 60,000 frames and one at 600,000 frames.",
        "Evidence bytes are logical receipt totals. Encoded bytes are reported separately from finalized recorder receipts.",
        "",
        "| Machine | Mode | Frames | Reps | Peak RSS median (MiB) | Runtime median (s) | Evidence logical (MiB) | Recorder encoded (MiB) |",
        "|---|---|---:|---:|---:|---:|---:|---:|",
    ]
    for row in summary.values():
        encoded = row["recorder_encoded_bytes_median"]
        encoded_text = f"{encoded / (1024 * 1024):.2f}" if encoded is not None else "—"
        lines.append(
            f"| {row['machine_id']} | {row['mode']} | {row['frame_limit']:,} | {row['repetitions']} | "
            f"{row['peak_rss_bytes_median'] / (1024 * 1024):.2f} | {row['process_wall_seconds_median']:.2f} | "
            f"{row['evidence_logical_bytes_median'] / (1024 * 1024):.2f} | {encoded_text} |"
        )
    lines.extend(("", "## Figures", "", "- `memory-vs-run-length.svg`", "- `evidence-volume-vs-run-length.svg`", ""))
    lines.extend(("## Interpretation", ""))
    for machine in machines:
        machine_rows = [row for row in summary.values() if row["machine_id"] == machine]
        ranges = []
        for mode in modes:
            mode_rows = sorted(
                (row for row in machine_rows if row["mode"] == mode),
                key=lambda row: row["frame_limit"],
            )
            if mode_rows:
                low = mode_rows[0]["peak_rss_bytes_median"] / (1024 * 1024)
                high = mode_rows[-1]["peak_rss_bytes_median"] / (1024 * 1024)
                ranges.append((mode, low, high))
        if ranges and all(high <= low * 1.5 for _, low, high in ranges):
            lines.append(
                f"{machine}: peak RSS stayed within approximately 1.5× between the shortest and longest measured runs in every mode. "
                + "; ".join(f"{mode} {low:.2f}→{high:.2f} MiB" for mode, low, high in ranges)
                + "."
            )
        else:
            lines.append(
                f"{machine}: peak RSS grew materially in at least one mode over the measured run lengths; inspect the table and source before making a bounded-memory claim."
            )
        lines.append("")
    if "chip8" in machines:
        lines.extend(
            (
                "The CHIP-8 cleanup removed retained per-frame hash/metric histories and whole-run native event accumulation. Its engine now keeps scalar event counters; selected frame-derived motifs are produced by the online bundle normalizer from requested frame artifacts. The measurement supports bounded observed RSS for these workloads and this host, not a general proof; allocator profiling was unavailable.",
                "",
            )
        )
    lines.extend(
        (
            "Recorder encoded bytes grow with run length and are reported separately from RSS. The 600,000-frame endpoint has one repetition; the smaller lengths have three. Results are specific to the listed fixtures/configurations and this WSL2 host, not a cross-host benchmark.",
            "",
        )
    )
    (root / "SUMMARY.md").write_text("\n".join(lines))
    print(f"Validated {len(trials)} trials; wrote summary and figures in {root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
