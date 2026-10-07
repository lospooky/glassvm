#!/usr/bin/env python3
"""Generate the concise Slice 4 pilot accounting report from raw trials."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def channel_value(receipt: dict, channel: str, key: str) -> int:
    for item in receipt.get("channels", []):
        if item.get("channel") == channel:
            value = item.get(key)
            return int(value) if value is not None else 0
    return 0


def render(results: list[dict]) -> str:
    lines = [
        "# Slice 4 pilot: accounting and lifecycle check",
        "",
        "These short, eight-frame runs validate harness wiring and receipt separation;",
        "they are not performance or bounded-memory evidence. Each run used the",
        "published 0.1.2 Python distributions in an isolated Python 3.14 environment.",
        "Recorder receipt schema 1.1.0 exposes per-channel persisted segment, record,",
        "block, logical-byte, and encoded-byte counters; the harness validates these",
        "against aggregate receipt totals without reading stored records.",
        "",
        "| Machine | Recipe | Frames | Evidence | Recorder | Published | Evidence records | Evidence logical bytes | Physical directory bytes | Per-channel recorder counters |",
        "| --- | --- | ---: | --- | --- | --- | ---: | ---: | ---: | --- |",
    ]
    for result in sorted(results, key=lambda item: (item["trial"]["machine_id"], item["trial"]["recipe_id"])):
        machine = result["trial"]["machine_id"]
        recipe = result["trial"]["recipe_id"]
        if result["kind"] == "excluded":
            lines.append(f"| {machine} | {recipe} | — | — | — | excluded | — | — | — | — |")
            continue
        if result["kind"] == "failed":
            phase = result.get("failure", {}).get("phase", "failed")
            lines.append(f"| {machine} | {recipe} | — | — | — | failed ({phase}) | — | — | — | — |")
            continue
        evidence_channels = result.get("evidence_channels", [])
        non_envelope = [item for item in evidence_channels if item.get("channel") != "envelope"]
        record_count = sum(int(item.get("item_count", 0)) for item in non_envelope)
        logical_bytes = sum(int(item.get("logical_bytes", 0)) for item in non_envelope)
        common = result["execution_result"]["common"]
        lines.append(
            "| {machine} | {recipe} | {frames} | {evidence} | {recorder} | {published} | "
            "{records} | {logical} | {physical} | {counters} |".format(
                machine=machine,
                recipe=recipe,
                frames=common.get("frames", "—"),
                evidence=result["evidence_receipt"].get("status", "—"),
                recorder=result["recorder_receipt"].get("status", "—"),
                published=result["publication"].get("published", False),
                records=record_count,
                logical=logical_bytes,
                physical=result["publication"].get("run_directory_bytes", "—"),
                counters="available" if result["recorder"].get("channel_stats_available") else "missing",
            )
        )
    lines.extend([
        "",
        "`Evidence logical bytes` and `physical directory bytes` have different",
        "measurement boundaries. The disabled rows still contain run envelope",
        "metadata; they request no evidence channels. All rows preserve machine,",
        "evidence, recorder, and publication outcomes as separate fields.",
        "",
    ])
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pilot-dir", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw_dir = args.pilot_dir / "raw"
    results = [json.loads(path.read_text()) for path in sorted(raw_dir.glob("*.json"))]
    if not results:
        parser.error(f"no raw run results found under {raw_dir}")
    output = args.pilot_dir / "PILOT_SUMMARY.md"
    rendered = render(results)
    if args.check:
        if not output.exists() or output.read_text() != rendered:
            raise SystemExit(f"{output} is stale; regenerate without --check")
    else:
        output.write_text(rendered)
        print(f"wrote {output}")


if __name__ == "__main__":
    main()
