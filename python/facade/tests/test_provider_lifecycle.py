"""Base-wheel smoke coverage for the generic GlassVM provider facade."""

import json
import unittest

import glassvm_py

class ProviderDiscoveryTests(unittest.TestCase):
    def setUp(self):
        self.runtime = glassvm_py.Runtime.discover()

    def test_base_runtime_discovers_entry_points_without_static_bundle_assumptions(self):
        metadata = json.loads(self.runtime.bundles())
        self.assertIsInstance(metadata, list)
        machine_ids = [entry["machine_id"] for entry in metadata]
        self.assertEqual(machine_ids, sorted(machine_ids))

        for entry in metadata:
            with self.subTest(machine_id=entry["machine_id"]):
                self.assertEqual(entry["protocol"], "glassvm.python_bundle")
                self.assertEqual(
                    entry["protocol_version"],
                    {"major": 1, "minor": 0, "patch": 0},
                )
                self.assertEqual(entry["prepare_function"], "prepare_run")
                self.assertEqual(entry["execute_function"], "execute_prepared")

    def test_unavailable_machine_fails_at_the_facade_boundary(self):
        with self.assertRaises(LookupError):
            self.runtime.prepare("glassvm.machine.not-installed", b"")


if __name__ == "__main__":
    unittest.main()
