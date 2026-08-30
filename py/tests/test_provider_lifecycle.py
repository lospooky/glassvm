"""Installed-wheel smoke coverage for the generic GlassVM provider facade."""

import json
import pathlib
import tempfile
import unittest

import glassvm_py


REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[3]


class ProviderLifecycleTests(unittest.TestCase):
    def setUp(self):
        self.runtime = glassvm_py.Runtime.discover()
        self.controls = {
            "frame_limit": 1,
            "step_limit": None,
            "bundle_limits": [],
        }

    def test_discovery_and_preparation_fail_before_execution_for_bad_inputs(self):
        with self.assertRaises(LookupError):
            self.runtime.prepare("missing", b"", execution_controls=self.controls)

        with self.assertRaises(ValueError):
            self.runtime.prepare("chip8", b"", execution_controls=self.controls)

        bad_configuration = {
            "schema": {
                "id": "chip8.machine_configuration",
                "version": {"major": 2, "minor": 0, "patch": 0},
            },
            "value": {"Map": {"unknown": {"Unsigned": 1}}},
        }
        with self.assertRaises(ValueError):
            self.runtime.prepare(
                "chip8",
                bytes([0x12, 0x00]),
                configuration=bad_configuration,
                execution_controls=self.controls,
            )

        bad_schedule = {
            "schema": {
                "id": "glassvm.invalid_input_schedule",
                "version": {"major": 1, "minor": 0, "patch": 0},
            },
            "entries": [],
        }
        with self.assertRaises(ValueError):
            self.runtime.prepare(
                "chip8",
                bytes([0x12, 0x00]),
                input_schedule=bad_schedule,
                execution_controls=self.controls,
            )

    def test_all_reference_providers_use_the_same_prepared_file_lifecycle(self):
        metadata = json.loads(self.runtime.bundles())
        self.assertTrue(
            all(
                entry["protocol"] == "glassvm.python_bundle"
                and entry["protocol_version"] == {"major": 1, "minor": 0, "patch": 0}
                and entry["prepare_function"] == "prepare_run"
                and entry["execute_function"] == "execute_prepared"
                for entry in metadata
            )
        )
        providers = {entry["machine_id"] for entry in metadata}
        self.assertTrue({"chip8", "hexwell", "wyrd16"}.issubset(providers))

        artifacts = {
            "chip8": (REPOSITORY_ROOT / "machines/chip8/fixtures/smoke.rom").read_bytes(),
            "hexwell": (REPOSITORY_ROOT / "machines/hexwell/fixtures/smoke.rom").read_bytes(),
            "wyrd16": (REPOSITORY_ROOT / "machines/wyrd16/fixtures/smoke.rom").read_bytes(),
        }
        for machine_id, artifact in artifacts.items():
            with self.subTest(machine_id=machine_id), tempfile.TemporaryDirectory() as directory:
                output_path = pathlib.Path(directory) / "run"
                prepared = self.runtime.prepare(
                    machine_id=machine_id,
                    artifact=artifact,
                    execution_controls=self.controls,
                )
                result = prepared.execute(str(output_path))

                self.assertIsInstance(result, dict)
                self.assertIn("execution", result)
                self.assertIn("evidence_receipt", result)
                self.assertIn("recorder_receipt", result)
                self.assertIn("published_run", result)
                self.assertTrue(result["recorder_receipt"]["finalized"])
                self.assertTrue(result["published_run"]["published"])
                self.assertTrue((output_path / "recorder-receipt.json").exists())


if __name__ == "__main__":
    unittest.main()
