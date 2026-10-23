from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch


RUNNER_PATH = Path(__file__).resolve().parents[1] / "run_paper.py"
SPEC = importlib.util.spec_from_file_location("glassvm_paper_runner", RUNNER_PATH)
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class PaperRunnerTests(unittest.TestCase):
    def test_release_pin_and_machine_set_are_explicit(self) -> None:
        self.assertEqual(runner.VERSION, "0.3.0")
        self.assertEqual(runner.MACHINES, ("chip8", "pico8", "tic80"))
        self.assertIn("glassvm-machines", runner.ALL_DISTRIBUTIONS)

    def test_default_results_are_outside_both_source_checkouts(self) -> None:
        self.assertNotEqual(
            runner.DEFAULT_RESULTS_ROOT,
            runner.GLASSVM_ROOT,
        )
        self.assertNotEqual(
            runner.DEFAULT_RESULTS_ROOT,
            runner.DEFAULT_MACHINES_ROOT,
        )

    def test_repeated_observation_matrix_uses_summary_manifest_name(self) -> None:
        args = SimpleNamespace(
            glassvm_root=Path("/glassvm"),
            machines_root=Path("/glassvm-machines"),
        )
        with patch.object(runner, "command") as command:
            runner.run_pilot(args, Path("/results/observation-matrix"), 5)

        pilot_argv = command.call_args_list[0].args[1]
        self.assertIn("--manifest-name", pilot_argv)
        manifest_index = pilot_argv.index("--manifest-name") + 1
        self.assertEqual(pilot_argv[manifest_index], "measurement_manifest.json")

    def test_paper_fixtures_rebuild_to_the_manifest_digests(self) -> None:
        runner.verify_fixtures(runner.DEFAULT_MACHINES_ROOT)


if __name__ == "__main__":
    unittest.main()
