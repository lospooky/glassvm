#!/usr/bin/env python3
"""Build bounded, machine-neutral Glass Garden postcards from live emissions.

The consumer retains fixed-size frame buckets, bounded normalized-kind
counters, one previous frame fingerprint, and one evidence receipt. It never
reads recorder storage or retains an event/frame history.
"""

from __future__ import annotations

import argparse
import html
import importlib.metadata
import json
import platform
import tomllib
from collections import Counter
from pathlib import Path
from typing import Any

BUCKET_COUNT = 20
TOP_EVENT_KINDS = 5
MAX_EVENT_KIND_COUNTERS = 128
MACHINES = ("chip8", "pico8", "tic80")
PAPER_ROOT = Path(__file__).resolve().parents[1]
OBSERVATION_REQUEST = {
    "schema_version": {"major": 2, "minor": 1, "patch": 0},
    "guarantee": "summary",
    "normalized_events": {
        "events": "all",
        "access_detail": "none",
        "state_diffs": False,
    },
    "capabilities": [],
    "native_evidence": {"enabled": False, "all": False, "kinds": []},
    "frames": {"capture": "hashes"},
    "snapshots": {"capture": "none"},
    "input_value_evidence": {"enabled": False},
    "budgets": {},
    "overflow": "fail",
}


class GlassGardenConsumer:
    """Incrementally summarize frame-hash changes and normalized event kinds."""

    def __init__(self, frame_limit: int) -> None:
        if frame_limit <= 0:
            raise ValueError("frame_limit must be positive")
        self.frame_limit = frame_limit
        self.frames_per_bucket = [0] * BUCKET_COUNT
        self.comparisons_per_bucket = [0] * BUCKET_COUNT
        self.changes_per_bucket = [0] * BUCKET_COUNT
        self.event_kind_counts: Counter[str] = Counter()
        self.total_normalized_events = 0
        self.frame_count = 0
        self.previous_fingerprint: bytes | None = None
        self.evidence_receipt: dict[str, Any] | None = None

    def emit_batch(self, emissions: list[dict[str, Any]]) -> None:
        for emission in emissions:
            kind = emission["kind"]
            record = emission["record"]
            if kind == "frame":
                self._observe_frame(record)
            elif kind == "event":
                self._observe_event(record)
            elif kind == "evidence_receipt":
                self.evidence_receipt = record
            elif kind in ("run_started", "run_finished"):
                continue
            else:
                raise ValueError(
                    f"Glass Garden did not request or understand emission {kind!r}"
                )

    def record_capability_outputs(self, outputs: list[dict[str, Any]]) -> None:
        if outputs:
            raise ValueError("Glass Garden did not request capability outputs")

    def record_capability_receipts(self, receipts: list[dict[str, Any]]) -> None:
        if receipts:
            raise ValueError("Glass Garden did not request capability receipts")

    def _observe_frame(self, record: dict[str, Any]) -> None:
        frame = record.get("frame")
        if not isinstance(frame, int) or frame < 0 or frame >= self.frame_limit:
            raise ValueError(f"frame coordinate {frame!r} is outside the requested limit")

        evidence = record.get("evidence")
        if not isinstance(evidence, dict) or set(evidence) != {"fingerprint"}:
            raise ValueError("Glass Garden requires fingerprint-only frame artifacts")
        fingerprint_value = evidence["fingerprint"]
        if not isinstance(fingerprint_value, dict) or not isinstance(
            fingerprint_value.get("bytes"), list
        ):
            raise TypeError("frame fingerprint must contain its typed bytes")
        fingerprint = bytes(fingerprint_value["bytes"])
        if len(fingerprint) != 32:
            raise ValueError("Glass Garden expects a 32-byte frame fingerprint")

        bucket = min(frame * BUCKET_COUNT // self.frame_limit, BUCKET_COUNT - 1)
        self.frames_per_bucket[bucket] += 1
        if self.previous_fingerprint is not None:
            self.comparisons_per_bucket[bucket] += 1
            if fingerprint != self.previous_fingerprint:
                self.changes_per_bucket[bucket] += 1
        self.previous_fingerprint = fingerprint
        self.frame_count += 1

    def _observe_event(self, record: dict[str, Any]) -> None:
        kind = record.get("kind")
        if not isinstance(kind, str) or not kind:
            raise ValueError("normalized event has no canonical kind")
        if kind not in self.event_kind_counts and len(self.event_kind_counts) >= MAX_EVENT_KIND_COUNTERS:
            raise ValueError(
                f"normalized event kind count exceeded the fixed limit "
                f"({MAX_EVENT_KIND_COUNTERS})"
            )
        self.event_kind_counts[kind] += 1
        self.total_normalized_events += 1

    @property
    def total_comparisons(self) -> int:
        return sum(self.comparisons_per_bucket)

    @property
    def total_changes(self) -> int:
        return sum(self.changes_per_bucket)

    @property
    def changed_buckets(self) -> int:
        return sum(count > 0 for count in self.changes_per_bucket)

    def top_event_kinds(self) -> list[tuple[str, int]]:
        return sorted(
            self.event_kind_counts.items(), key=lambda pair: (-pair[1], pair[0])
        )[:TOP_EVENT_KINDS]


def _evidence_status(consumer: GlassGardenConsumer) -> str:
    if consumer.evidence_receipt is None:
        raise RuntimeError("the consumer did not receive an EvidenceReceipt")
    status = consumer.evidence_receipt.get("status")
    if status not in {"complete", "incomplete", "failed"}:
        raise ValueError(f"unknown EvidenceReceipt status: {status!r}")
    return status


def _event_text(consumer: GlassGardenConsumer) -> str:
    top = consumer.top_event_kinds()
    if not top:
        return "No normalized events"
    return "; ".join(f"{kind} × {count}" for kind, count in top)


def render_postcard(machine_id: str, consumer: GlassGardenConsumer) -> str:
    """Render a static SVG whose fireflies encode literal hash-change rates."""

    status = _evidence_status(consumer)
    width, height = 1000, 390
    left, step = 58, 46
    fireflies: list[str] = []
    labels: list[str] = []
    for index in range(BUCKET_COUNT):
        x = left + index * step
        comparisons = consumer.comparisons_per_bucket[index]
        changes = consumer.changes_per_bucket[index]
        rate = changes / comparisons if comparisons else 0.0
        radius = 5 + 5 * rate
        opacity = 0.18 + 0.78 * rate if comparisons else 0.14
        start = index * consumer.frame_limit // BUCKET_COUNT
        end = (index + 1) * consumer.frame_limit // BUCKET_COUNT - 1
        fireflies.append(
            f'<path d="M{x} 163 Q{x - 5} 146 {x} 129" fill="none" '
            f'stroke="#76a98b" stroke-width="2"/>'
            f'<circle cx="{x}" cy="126" r="{radius + 7:.2f}" '
            f'fill="#f4b942" opacity="{opacity * 0.34:.3f}"/>'
            f'<circle cx="{x}" cy="126" r="{radius:.2f}" '
            f'fill="#f4b942" opacity="{opacity:.3f}"/>'
            f'<circle cx="{x}" cy="126" r="2" fill="#fff9d7"/>'
        )
        labels.append(
            f'<text x="{x}" y="190" text-anchor="middle" font-family="sans-serif" '
            f'font-size="10" fill="#718176">'
            f"{start}–{end}</text>"
        )

    top_kinds = consumer.top_event_kinds()
    event_rows: list[str] = []
    max_count = max((count for _, count in top_kinds), default=1)
    for index, (kind, count) in enumerate(top_kinds):
        y = 258 + index * 25
        safe_kind = html.escape(kind, quote=True)
        bar_width = 210 * count / max_count
        event_rows.append(
            f'<text x="60" y="{y}" font-family="monospace" font-size="12" '
            f'fill="#35483e">{safe_kind}</text>'
            f'<rect x="520" y="{y - 12}" width="{bar_width:.1f}" height="12" '
            f'rx="6" fill="#76a98b"/>'
            f'<text x="745" y="{y}" font-family="sans-serif" font-size="12" '
            f'fill="#35483e">{count}</text>'
        )
    if not event_rows:
        event_rows.append(
            '<text x="60" y="258" font-family="monospace" font-size="12" '
            'fill="#35483e">None</text>'
        )

    status_label = {
        "complete": "Evidence fulfilled",
        "incomplete": "Evidence incomplete",
        "failed": "Evidence failed",
    }[status]
    status_color = {
        "complete": "#d9eee0",
        "incomplete": "#fff0c2",
        "failed": "#f6d6d1",
    }[status]
    changed_percent = (
        100 * consumer.total_changes / consumer.total_comparisons
        if consumer.total_comparisons
        else 0.0
    )

    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title description">
  <title id="title">Glass Garden — {html.escape(machine_id)} run postcard</title>
  <desc id="description">Each firefly summarizes changed adjacent frame fingerprints divided by comparisons in its frame-coordinate bucket. Event kinds are counted separately.</desc>
  <rect x="0" y="0" width="{width}" height="{height}" rx="24" fill="#fbf8ef"/>
  <text x="48" y="52" font-family="sans-serif" font-size="28" font-weight="700" fill="#264c3d">Glass Garden</text>
  <text x="49" y="78" font-family="sans-serif" font-size="15" font-weight="600" fill="#60796b">{html.escape(machine_id)}</text>
  <rect x="754" y="34" width="196" height="34" rx="17" fill="{status_color}"/>
  <text x="852" y="56" text-anchor="middle" font-family="sans-serif" font-size="12" font-weight="700" fill="#315541">{status_label}</text>
  <text x="48" y="108" font-family="sans-serif" font-size="13" fill="#53685c">Frame-hash changes by frame coordinate — 20 fixed buckets</text>
  <path d="M55 164 Q510 177 945 164" fill="none" stroke="#b7d0bd" stroke-width="3" stroke-linecap="round"/>
  {''.join(fireflies)}
  {''.join(labels)}
  <text x="48" y="218" font-family="sans-serif" font-size="13" fill="#53685c">{consumer.total_changes} of {consumer.total_comparisons} adjacent fingerprint pairs changed ({changed_percent:.1f}%); {consumer.changed_buckets} buckets contained a change.</text>
  <text x="48" y="244" font-family="sans-serif" font-size="14" font-weight="700" fill="#315541">Top normalized event kinds — counts only</text>
  {''.join(event_rows)}
</svg>
'''


def render_summary(rows: list[dict[str, Any]], package_versions: dict[str, str]) -> str:
    lines = [
        "# Glass Garden",
        "",
        "A small downstream consumer attached directly to each run's Python emission stream.",
        "It requests frame hashes and normalized events only; it does not read recorder files or retain event/frame history.",
        "",
        f"Run environment: CPython {platform.python_version()}; "
        + ", ".join(f"{name} {version}" for name, version in package_versions.items())
        + ".",
        "Each bundle supplies its own declared default configuration. The caller supplies an empty input schedule and the shared 600-frame limit.",
        "",
        "Each firefly summarizes the number of changed adjacent frame fingerprints divided by the number compared in that frame-coordinate bucket. A pair is attributed to the bucket of its later delivered frame artifact. Normalized event kinds are counted separately; no cross-channel ordering is inferred.",
        "",
        "| Machine | Frame buckets | Changed pairs (bucketed) | Top normalized event kinds | Evidence status | Postcard |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for row in rows:
        top_parts = []
        for kind, count in row["top_event_kinds"]:
            escaped_kind = kind.replace("`", "\\`")
            top_parts.append(f"`{escaped_kind}` × {count}")
        top = "; ".join(top_parts) or "—"
        lines.append(
            f"| {row['machine_id']} | {row['observed_buckets']}/{BUCKET_COUNT} "
            f"({row['frames']} frames) | {row['changes']}/{row['comparisons']} "
            f"({row['changed_buckets']} buckets changed) | {top} | "
            f"{row['evidence_status']} | [`SVG`]({row['svg_name']}) |"
        )
    lines.extend(
        [
            "",
            "These postcards are descriptive summaries of the selected evidence, not performance scores, behavioral-equivalence claims, or measures of program complexity.",
            "",
        ]
    )
    return "\n".join(lines)


def run_machine(
    runtime: Any,
    machine_id: str,
    workload: dict[str, Any],
    machines_root: Path,
    frame_limit: int,
) -> tuple[GlassGardenConsumer, dict[str, Any]]:
    artifact_path = machines_root / workload["artifact_path"]
    artifact = artifact_path.read_bytes()
    controls = {"frame_limit": frame_limit, "step_limit": None, "bundle_limits": []}
    prepared = runtime.prepare(
        machine_id,
        artifact,
        execution_controls=controls,
        observation=OBSERVATION_REQUEST,
    )
    consumer = GlassGardenConsumer(frame_limit)
    result = prepared.execute(sinks=[consumer])
    if result["python_sink_delivery"]["status"] != "complete":
        raise RuntimeError(
            f"{machine_id}: Python sink delivery was not complete: "
            f"{result['python_sink_delivery']}"
        )
    if consumer.evidence_receipt != result["evidence_fulfillment"]:
        raise RuntimeError(f"{machine_id}: consumer receipt differs from run result")
    status = _evidence_status(consumer)
    row = {
        "machine_id": machine_id,
        "frames": consumer.frame_count,
        "observed_buckets": sum(count > 0 for count in consumer.frames_per_bucket),
        "comparisons": consumer.total_comparisons,
        "changes": consumer.total_changes,
        "changed_buckets": consumer.changed_buckets,
        "top_event_kinds": consumer.top_event_kinds(),
        "evidence_status": status,
        "svg_name": f"glass-garden-{machine_id}.svg",
    }
    return consumer, row


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--machines-root",
        type=Path,
        required=True,
        help="checkout root containing the three public paper fixtures",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        required=True,
        help="directory for three SVG postcards and SUMMARY.md",
    )
    args = parser.parse_args()

    workloads = tomllib.loads((PAPER_ROOT / "PAPER_WORKLOADS.toml").read_text())
    by_machine = {item["machine_id"]: item for item in workloads["workload"]}
    if set(by_machine) != set(MACHINES):
        raise ValueError(f"paper workload manifest must contain exactly {MACHINES}")
    if workloads["input_schedule"] != "empty" or workloads["frame_capture"] != "hashes":
        raise ValueError("Glass Garden requires the manifest's empty schedule and hash capture")

    import glassvm

    runtime = glassvm.Runtime.discover()
    discovered = {item["machine_id"] for item in json.loads(runtime.bundles())}
    missing = set(MACHINES) - discovered
    if missing:
        raise RuntimeError(
            "install all GlassVM machine providers before running Glass Garden; "
            f"missing entry points: {', '.join(sorted(missing))}"
        )

    args.output_dir.mkdir(parents=True, exist_ok=True)
    package_versions = {
        name: importlib.metadata.version(name)
        for name in ("glassvm", "glassvm-chip8", "glassvm-pico8", "glassvm-tic80")
    }
    rows: list[dict[str, Any]] = []
    for machine_id in MACHINES:
        consumer, row = run_machine(
            runtime,
            machine_id,
            by_machine[machine_id],
            args.machines_root,
            int(workloads["frame_limit"]),
        )
        (args.output_dir / row["svg_name"]).write_text(
            render_postcard(machine_id, consumer), encoding="utf-8"
        )
        rows.append(row)

    (args.output_dir / "SUMMARY.md").write_text(
        render_summary(rows, package_versions), encoding="utf-8"
    )
    print(args.output_dir / "SUMMARY.md")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
