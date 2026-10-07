"""Pure contract-shape tests for the Slice 4 experiment driver."""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("run_trial", HERE / "run_trial.py")
assert SPEC and SPEC.loader
RUN_TRIAL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUN_TRIAL)
PILOT_SPEC = importlib.util.spec_from_file_location("run_pilot", HERE / "run_pilot.py")
assert PILOT_SPEC and PILOT_SPEC.loader
RUN_PILOT = importlib.util.module_from_spec(PILOT_SPEC)
PILOT_SPEC.loader.exec_module(RUN_PILOT)


def workload(machine_id: str) -> dict:
    return {
        "machine_id": machine_id,
        "capability": f"{machine_id}.visual",
        "normalized_event_kinds": ["frame_completed", "extension:tic80.trace"],
    }


class TrialContractTests(unittest.TestCase):
    def test_structured_configuration_uses_typed_values_and_schema(self):
        encoded = RUN_TRIAL.machine_configuration("pico8", {"machine_seed": 0, "runtime": "lua"})
        self.assertEqual(encoded["schema"]["id"], "pico8.machine_configuration")
        self.assertEqual(encoded["schema"]["version"], {"major": 1, "minor": 0, "patch": 0})
        self.assertEqual(encoded["value"]["Map"]["machine_seed"], {"Unsigned": 0})
        self.assertEqual(encoded["value"]["Map"]["runtime"], {"Text": "lua"})

    def test_empty_schedule_and_frame_capture_are_independent_of_event_selection(self):
        recipe = {"normalized": "none", "frames": "hashes", "native": "none", "capability": False}
        request = RUN_TRIAL.observation_request(recipe, workload("chip8"))
        self.assertEqual(request["normalized_events"]["events"], "none")
        self.assertEqual(request["frames"]["capture"], "hashes")
        self.assertEqual(RUN_TRIAL.INPUT_SCHEDULE_SCHEMA["id"], "glassvm.input_schedule")

    def test_extension_event_uses_explicit_enum_tag(self):
        recipe = {"normalized": "workload", "frames": "hashes", "native": "none", "capability": True}
        request = RUN_TRIAL.observation_request(recipe, workload("tic80"))
        self.assertEqual(request["normalized_events"]["events"]["kinds"], [
            "frame_completed", {"extension": "tic80.trace"}
        ])
        self.assertEqual(request["capabilities"][0]["id"], "tic80.visual")

    def test_full_capture_recipe_does_not_smuggle_in_normalized_events(self):
        recipe = {"normalized": "none", "frames": "full", "native": "none", "capability": False}
        request = RUN_TRIAL.observation_request(recipe, workload("pico8"))
        self.assertEqual(request["normalized_events"]["events"], "none")
        self.assertEqual(request["frames"]["capture"], "full")

    def test_result_schema_is_versioned_and_has_separate_outcome_domains(self):
        schema = json.loads((HERE / "result.schema.json").read_text())
        self.assertEqual(schema["$id"], "glassvm.paper-experiment-result/1.0.0")
        self.assertTrue({"execution_result", "evidence_receipt", "recorder_receipt"} <= set(schema["properties"]))
        self.assertTrue({"outcomes", "publication", "recorder"} <= set(schema["properties"]))

    def test_failed_preparation_is_a_distinct_recorded_trial_outcome(self):
        result = {
            "result_schema_version": "1.0.0", "kind": "failed", "trial": {},
            "source": {}, "host": {}, "timing": {}, "workload": {}, "request": {},
            "negotiation": {}, "outcomes": {}, "evidence_channels": [], "recorder": {},
            "memory": {}, "publication": {"published": False},
            "failure": {"phase": "preparation", "error_type": "ValueError", "message": "invalid request"},
        }
        self.assertEqual(RUN_PILOT.validate_result(result)["failure"]["phase"], "preparation")

    def test_per_channel_accounting_is_checked_only_against_recorder_receipt(self):
        result = {
            "result_schema_version": "1.0.0", "kind": "run", "trial": {},
            "source": {}, "host": {}, "timing": {}, "workload": {}, "request": {},
            "negotiation": {}, "outcomes": {}, "evidence_channels": [],
            "recorder": {"status": "complete", "channel_stats_available": True},
            "memory": {}, "publication": {"published": True},
            "execution_result": {}, "evidence_receipt": {},
            "recorder_receipt": {
                "schema": RUN_PILOT.RECORDER_RECEIPT_SCHEMA,
                "status": "complete", "segment_count": 1, "record_count": 2,
                "block_count": 1, "logical_bytes": 20, "encoded_bytes": 16,
                "channels": [{"stats": {
                    "segment_count": 1, "record_count": 2, "block_count": 1,
                    "logical_bytes": 20, "encoded_bytes": 16,
                }}],
            },
        }
        RUN_PILOT.validate_result(result)
        result["recorder_receipt"]["record_count"] = 3
        with self.assertRaisesRegex(ValueError, "record_count"):
            RUN_PILOT.validate_result(result)

    def test_pilot_requires_the_published_counter_schema(self):
        result = {
            "result_schema_version": "1.0.0", "kind": "run", "trial": {},
            "source": {}, "host": {}, "timing": {}, "workload": {},
            "request": {}, "negotiation": {}, "outcomes": {}, "evidence_channels": [],
            "recorder": {"status": "complete", "channel_stats_available": False},
            "memory": {}, "publication": {"published": True},
            "execution_result": {}, "evidence_receipt": {},
            "recorder_receipt": {"schema": {"id": "glassvm.recorder_receipt"}},
        }
        with self.assertRaisesRegex(ValueError, "expected recorder receipt schema 1.1.0"):
            RUN_PILOT.validate_result(result)

    def test_pilot_checks_per_channel_recorder_totals_without_reconstructing_records(self):
        result = {
            "result_schema_version": "1.0.0",
            "kind": "run",
            "trial": {}, "source": {}, "host": {}, "timing": {}, "workload": {},
            "request": {}, "negotiation": {}, "outcomes": {}, "evidence_channels": [],
            "recorder": {"status": "complete", "channel_stats_available": True},
            "memory": {}, "publication": {"published": True},
            "execution_result": {}, "evidence_receipt": {},
            "recorder_receipt": {
                "schema": RUN_PILOT.RECORDER_RECEIPT_SCHEMA,
                "status": "complete",
                "segment_count": 1, "record_count": 2, "block_count": 1,
                "logical_bytes": 20, "encoded_bytes": 16,
                "channels": [{"stats": {
                    "segment_count": 1, "record_count": 2, "block_count": 1,
                    "logical_bytes": 20, "encoded_bytes": 16,
                }}],
            },
        }
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "result.json"
            path.write_text(json.dumps(result))
            self.assertEqual(RUN_PILOT.validate_trial(path)["kind"], "run")
            result["recorder_receipt"]["record_count"] = 3
            path.write_text(json.dumps(result))
            with self.assertRaisesRegex(ValueError, "record_count"):
                RUN_PILOT.validate_trial(path)


if __name__ == "__main__":
    unittest.main()
