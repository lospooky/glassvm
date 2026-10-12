"""Pure bounded-state tests for the Glass Garden downstream consumer."""

from __future__ import annotations

import importlib.util
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("glass_garden.py")
SPEC = importlib.util.spec_from_file_location("glass_garden", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
glass_garden = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(glass_garden)


def frame(frame_id: int, fingerprint: bytes) -> dict:
    return {
        "kind": "frame",
        "record": {
            "frame": frame_id,
            "evidence": {"fingerprint": {"bytes": list(fingerprint)}},
        },
    }


class GlassGardenTests(unittest.TestCase):
    def test_request_selects_only_hashes_and_normalized_events(self):
        request = glass_garden.OBSERVATION_REQUEST
        self.assertEqual(request["normalized_events"]["events"], "all")
        self.assertEqual(request["frames"]["capture"], "hashes")
        self.assertEqual(request["native_evidence"]["enabled"], False)
        self.assertEqual(request["capabilities"], [])
        self.assertEqual(request["snapshots"]["capture"], "none")
        self.assertEqual(request["input_value_evidence"]["enabled"], False)

    def test_buckets_count_only_adjacent_frame_fingerprint_changes(self):
        consumer = glass_garden.GlassGardenConsumer(frame_limit=20)
        for frame_id in range(20):
            digest = bytes([frame_id % 2]) * 32
            consumer.emit_batch([frame(frame_id, digest)])

        self.assertEqual(consumer.frame_count, 20)
        self.assertEqual(consumer.total_comparisons, 19)
        self.assertEqual(consumer.total_changes, 19)
        self.assertEqual(consumer.changed_buckets, 19)
        self.assertEqual(sum(consumer.frames_per_bucket), 20)

    def test_event_kind_counts_are_bounded_and_top_n_is_deterministic(self):
        consumer = glass_garden.GlassGardenConsumer(frame_limit=20)
        for kind in ("event.b", "event.a", "event.a", "event.c", "event.a"):
            consumer.emit_batch([{"kind": "event", "record": {"kind": kind}}])
        self.assertEqual(
            consumer.top_event_kinds(),
            [("event.a", 3), ("event.b", 1), ("event.c", 1)],
        )

        consumer.event_kind_counts = glass_garden.Counter(
            {f"event.{index}": 1 for index in range(glass_garden.MAX_EVENT_KIND_COUNTERS)}
        )
        with self.assertRaisesRegex(ValueError, "fixed limit"):
            consumer.emit_batch(
                [{"kind": "event", "record": {"kind": "event.overflow"}}]
            )

    def test_svg_labels_literal_change_ratio_and_receipt_status(self):
        consumer = glass_garden.GlassGardenConsumer(frame_limit=2)
        consumer.emit_batch(
            [
                frame(0, b"a" * 32),
                frame(1, b"b" * 32),
                {
                    "kind": "evidence_receipt",
                    "record": {"status": "complete"},
                },
            ]
        )
        svg = glass_garden.render_postcard("chip8", consumer)
        root = ET.fromstring(svg)
        self.assertEqual(root.tag, "{http://www.w3.org/2000/svg}svg")
        self.assertIn("1 of 1 adjacent fingerprint pairs changed", svg)
        self.assertIn("Evidence fulfilled", svg)

    def test_full_frame_payload_is_rejected(self):
        consumer = glass_garden.GlassGardenConsumer(frame_limit=1)
        with self.assertRaisesRegex(ValueError, "fingerprint-only"):
            consumer.emit_batch(
                [{"kind": "frame", "record": {"frame": 0, "evidence": {"full": {"bytes": []}}}}]
            )


if __name__ == "__main__":
    unittest.main()
