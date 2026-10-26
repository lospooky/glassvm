"""Check attribution and uncertainty in the generic downstream protocol tester."""

from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path

PATH = Path(__file__).with_name("glass_relay.py")
SPEC = importlib.util.spec_from_file_location("glass_relay", PATH)
assert SPEC and SPEC.loader
relay = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(relay)

PROTOCOL = json.loads(relay.EXPERIMENT.read_text())
INPUT_SCHEMA = {"id": "test.action", "version": relay.V1}


def input_event(frame: int, *, shift: int = 0) -> dict:
    coordinate = frame + shift
    return {"kind": "event", "record": {
        "kind": "input_applied", "frame": coordinate, "extensions": {"glassvm.input_applied": {
            "input_id": "test.action", "input_schema": INPUT_SCHEMA,
            "source": {"kind": "scheduled", "ordinal": frame},
            "application_coordinate": {"kind": "frame", "value": {"frame": coordinate}},
        }},
    }}


def frame_event(frame: int, active: bool) -> dict:
    return {"kind": "frame", "record": {
        "frame": frame, "representation_schema": {"id": "test.fingerprint", "version": relay.V1},
        "evidence": {"fingerprint": {"bytes": list((b"a" if active else b"b") * 32)}},
    }}


def evaluate(mode: str = "reference", *, frames_first=False, shift=0, evidence="complete") -> dict:
    consumer = relay.RelayConsumer(PROTOCOL, "test.action", INPUT_SCHEMA)
    consumer.emit_batch([{"kind": "run_started", "record": {
        "prepared_run_identity": "prepared-execution", "prepared_observation_id": "prepared-observation"}}])
    inputs, frames = [], []
    for frame in range(PROTOCOL["frame_limit"]):
        active = any(trial["press"] + (3 if mode == "delayed" else 0) <= frame < trial["release"]
                     for trial in PROTOCOL["trials"])
        if mode == "stuck":
            active = frame >= PROTOCOL["trials"][0]["press"]
        if mode == "silent":
            active = False
        inputs.append(input_event(frame, shift=shift))
        frames.append(frame_event(frame, active))
    if frames_first:
        consumer.emit_batch(frames)
        consumer.emit_batch(inputs)
    else:
        for input_record, frame_record in zip(inputs, frames):
            consumer.emit_batch([input_record, frame_record])
    receipt = {"status": evidence}
    consumer.emit_batch([{"kind": "run_finished", "record": {}},
                         {"kind": "evidence_receipt", "record": receipt}])
    return consumer.finish({"machine_execution": {"status": "completed"},
                            "evidence_fulfillment": receipt,
                            "python_sink_delivery": {"status": "complete"},
                            "recorder_finalization": {"status": "not_requested"},
                            "publication": {"status": "not_published"}})


class RelayTests(unittest.TestCase):
    def test_cross_channel_delivery_order_does_not_change_measurements(self):
        normal = evaluate()
        reversed_channels = evaluate(frames_first=True)
        self.assertEqual(normal["status"], "pass")
        self.assertEqual(normal["trials"], reversed_channels["trials"])

    def test_delay_is_measured_from_acceptance_frame_not_callback_order(self):
        result = evaluate("delayed", frames_first=True)
        self.assertEqual(result["status"], "fail")
        self.assertEqual([trial["response_delay_frames"] for trial in result["trials"]], [3, 3, 3])
        self.assertTrue(all(trial["failures"] == ["response_deadline"] for trial in result["trials"]))

    def test_persistent_output_is_not_a_new_response_to_later_inputs(self):
        result = evaluate("stuck")
        self.assertIn("not_restored", result["trials"][0]["failures"])
        self.assertTrue(all(trial["response_delay_frames"] is None for trial in result["trials"][1:]))
        self.assertTrue(all(not trial["baseline_at_press"] for trial in result["trials"][1:]))

    def test_missing_evidence_cannot_be_called_a_behavioral_failure(self):
        result = evaluate("silent", evidence="incomplete")
        self.assertEqual(result["status"], "inconclusive")
        self.assertTrue(all(trial["failures"] == [] for trial in result["trials"]))
        self.assertEqual(result["outcomes"]["machine_execution"]["status"], "completed")
        self.assertEqual(result["outcomes"]["python_sink_delivery"]["status"], "complete")

    def test_absent_response_cannot_be_called_a_successful_restoration(self):
        result = evaluate("silent")
        self.assertEqual(result["status"], "fail")
        self.assertTrue(all(trial["failures"] == ["no_visible_response"] for trial in result["trials"]))
        self.assertTrue(all(trial["restoration_delay_frames"] is None for trial in result["trials"]))

    def test_shifted_acceptance_withholds_response_measurements(self):
        result = evaluate(shift=1)
        self.assertEqual(result["status"], "inconclusive")
        self.assertGreater(result["stream_error_count"], 0)
        self.assertTrue(all(trial["response_delay_frames"] is None for trial in result["trials"]))

    def test_frame_gap_rejects_unobserved_response_intervals(self):
        consumer = relay.RelayConsumer(PROTOCOL, "test.action", INPUT_SCHEMA)
        consumer.emit_batch([frame_event(0, False)])
        with self.assertRaisesRegex(ValueError, "contiguous"):
            consumer.emit_batch([frame_event(2, True)])


if __name__ == "__main__":
    unittest.main()
