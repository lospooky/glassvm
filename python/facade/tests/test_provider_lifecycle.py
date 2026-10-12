"""Base-wheel smoke coverage for the generic GlassVM provider facade."""

import json
import sys
import types
import unittest
from unittest.mock import patch

import glassvm


class ProviderDiscoveryTests(unittest.TestCase):
    def setUp(self):
        self.runtime = glassvm.Runtime.discover()

    def test_base_runtime_discovers_entry_points_without_static_bundle_assumptions(
        self,
    ):
        metadata = json.loads(self.runtime.bundles())
        self.assertIsInstance(metadata, list)
        machine_ids = [entry["machine_id"] for entry in metadata]
        self.assertEqual(machine_ids, sorted(machine_ids))

        for entry in metadata:
            with self.subTest(machine_id=entry["machine_id"]):
                self.assertEqual(entry["protocol"], "glassvm.python_bundle")
                self.assertEqual(
                    entry["protocol_version"],
                    {"major": 2, "minor": 0, "patch": 0},
                )
                self.assertEqual(entry["prepare_function"], "prepare_run")
                self.assertEqual(entry["execute_function"], "execute_prepared")

    def test_unavailable_machine_fails_at_the_facade_boundary(self):
        with self.assertRaises(LookupError):
            self.runtime.prepare("glassvm.machine.not-installed", b"")

    def test_prepared_execution_forwards_python_consumers_and_optional_recorder(self):
        calls = {}

        def provider():
            return json.dumps(
                {
                    "protocol": "glassvm.python_bundle",
                    "protocol_version": {"major": 2, "minor": 0, "patch": 0},
                    "machine_id": "test-machine",
                    "distribution": "glassvm-test-machine",
                    "module": "glassvm_test_provider",
                    "prepare_function": "prepare_run",
                    "execute_function": "execute_prepared",
                    "bundle_version": "0.2.0",
                }
            )

        def prepare_run(*args):
            calls["prepared_args"] = args
            return object()

        def execute_prepared(prepared, sinks, output_path):
            calls["execute_args"] = (prepared, sinks, output_path)
            return {
                "machine_execution": {"status": "completed", "result": {}},
                "evidence_fulfillment": {"status": "complete"},
                "python_sink_delivery": {"status": "complete"},
                "recorder_finalization": {"status": "finalized", "result": {}},
                "publication": {"status": "published"},
                "published_run": {"published": True, "path": "runs/test"},
            }

        module = types.ModuleType("glassvm_test_provider")
        module.prepare_run = prepare_run
        module.execute_prepared = execute_prepared

        class EntryPoint:
            name = "test-machine"

            @staticmethod
            def load():
                return provider

        class Consumer:
            def emit_batch(self, emissions):
                pass

            def record_capability_outputs(self, outputs):
                pass

            def record_capability_receipts(self, receipts):
                pass

        with (
            patch("importlib.metadata.entry_points", return_value=[EntryPoint()]),
            patch.dict(sys.modules, {"glassvm_test_provider": module}),
        ):
            runtime = glassvm.Runtime.discover()
            prepared = runtime.prepare("test-machine", b"artifact")
            sink = Consumer()
            result = prepared.execute(
                sinks=[sink], file_recorder=glassvm.FileRecorder("runs/test")
            )

        self.assertEqual(calls["execute_args"][1], [sink])
        self.assertEqual(calls["execute_args"][2], "runs/test")
        self.assertEqual(result["machine_execution"]["status"], "completed")
        self.assertEqual(result["recorder_finalization"]["status"], "finalized")
        self.assertEqual(result["publication"]["status"], "published")

    def test_execute_accepts_no_consumers_and_no_file_recorder(self):
        def provider():
            return json.dumps(
                {
                    "protocol": "glassvm.python_bundle",
                    "protocol_version": {"major": 2, "minor": 0, "patch": 0},
                    "machine_id": "test-machine",
                    "distribution": "glassvm-test-machine",
                    "module": "glassvm_test_provider",
                    "prepare_function": "prepare_run",
                    "execute_function": "execute_prepared",
                    "bundle_version": "0.2.0",
                }
            )

        module = types.ModuleType("glassvm_test_provider")
        module.prepare_run = lambda *_args: object()

        def execute_prepared(_prepared, sinks, output_path):
            self.assertEqual(sinks, [])
            self.assertIsNone(output_path)
            return {
                "machine_execution": {"status": "completed", "result": {}},
                "evidence_fulfillment": {"status": "complete"},
                "python_sink_delivery": {"status": "not_requested"},
                "recorder_finalization": {"status": "not_requested"},
                "publication": {"status": "not_published"},
                "published_run": None,
            }

        module.execute_prepared = execute_prepared

        class EntryPoint:
            name = "test-machine"

            @staticmethod
            def load():
                return provider

        with (
            patch("importlib.metadata.entry_points", return_value=[EntryPoint()]),
            patch.dict(sys.modules, {"glassvm_test_provider": module}),
        ):
            prepared = glassvm.Runtime.discover().prepare("test-machine", b"artifact")
            result = prepared.execute()

        self.assertEqual(result["python_sink_delivery"]["status"], "not_requested")
        self.assertEqual(result["recorder_finalization"]["status"], "not_requested")
        self.assertIsNone(result["published_run"])

    def test_file_recorder_requires_a_non_empty_path(self):
        with self.assertRaises(ValueError):
            glassvm.FileRecorder("")


if __name__ == "__main__":
    unittest.main()
