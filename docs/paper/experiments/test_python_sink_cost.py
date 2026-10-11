"""Pure contract tests for the Python consumer sink-cost harness."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location(
    "run_python_sink_cost", HERE / "run_python_sink_cost.py"
)
assert SPEC and SPEC.loader
HARNESS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HARNESS)


class PythonSinkCostTests(unittest.TestCase):
    def test_all_five_modes_are_distinct(self):
        self.assertEqual(
            HARNESS.MODES,
            (
                "observation_disabled",
                "observed_no_external_sink",
                "python_consumer",
                "rust_recorder",
                "python_consumer_and_rust_recorder",
            ),
        )

    def test_summary_uses_median_and_no_external_consumer_reference(self):
        rows = []
        for mode, values in {
            "observed_no_external_sink": [2.0, 4.0, 3.0],
            "python_consumer": [3.0, 5.0, 4.0],
        }.items():
            for repetition, value in enumerate(values, start=1):
                rows.append(
                    {
                        "trial": {
                            "machine_id": "chip8",
                            "mode": mode,
                            "repetition": repetition,
                        },
                        "timing": {"execute_lifecycle_seconds": value},
                        "request": {"observation": "selected"},
                        "execution": {"status": "completed"},
                        "evidence_fulfillment": {"status": "complete"},
                        "identity": {
                            "prepared_observation_id": "observation-id",
                            "run_id": "run-id",
                        },
                    }
                )
        summary = HARNESS.summarize(rows)["chip8"]
        self.assertEqual(
            summary["observed_no_external_sink"]["median_execute_lifecycle_seconds"],
            3.0,
        )
        self.assertEqual(
            summary["python_consumer"][
                "median_delta_vs_observed_no_external_sink_seconds"
            ],
            1.0,
        )

    def test_summary_rejects_cross_mode_execution_drift(self):
        rows = [
            {
                "trial": {
                    "machine_id": "chip8",
                    "mode": mode,
                    "repetition": 1,
                },
                "request": {"observation": "same"},
                "execution": {
                    "status": "completed",
                    "result": {
                        "common": {
                            "frames": 10 if mode == "observed_no_external_sink" else 11
                        },
                        "capabilities": [],
                    },
                },
                "evidence_fulfillment": {"status": "complete"},
                "identity": {
                    "prepared_observation_id": "same",
                    "run_id": "run-1",
                },
                "timing": {"execute_lifecycle_seconds": 1.0},
            }
            for mode in ("observed_no_external_sink", "python_consumer")
        ]
        with self.assertRaisesRegex(ValueError, "execution differs"):
            HARNESS.summarize(rows)

    def test_counting_consumer_does_not_retain_emission_batches(self):
        consumer = HARNESS.CountingConsumer()
        consumer.emit_batch(
            [
                {"kind": "run_started", "record": {"run_id": "test-run"}},
                {"kind": "event", "record": {"kind": "frame_completed"}},
            ]
        )
        consumer.record_capability_outputs([{}])
        consumer.record_capability_receipts([{}])
        self.assertEqual(consumer.batch_count, 1)
        self.assertEqual(consumer.item_count, 2)
        self.assertEqual(consumer.capability_output_count, 1)
        self.assertEqual(consumer.capability_receipt_count, 1)
        self.assertEqual(consumer.run_started["run_id"], "test-run")
        self.assertFalse(hasattr(consumer, "emissions"))


if __name__ == "__main__":
    unittest.main()
