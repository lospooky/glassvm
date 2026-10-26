#!/usr/bin/env python3
"""Glass Relay: one bounded Python tester for a shared interactive protocol."""

from __future__ import annotations

import argparse
import hashlib
import html
import importlib.metadata
import json
import platform
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

EXPERIMENT = Path(__file__).with_name("glass_relay") / "experiment.json"
MAX_TRIALS = 8
MAX_STREAM_ERRORS = 16
PROTOCOL_KEYS = ("schema", "version", "frame_limit", "repetitions", "response_deadline_frames",
                 "restoration_deadline_frames", "trials")
V1 = {"major": 1, "minor": 0, "patch": 0}
OBSERVATION = {
    "schema_version": {"major": 2, "minor": 1, "patch": 0},
    "guarantee": "summary",
    "normalized_events": {"events": {"kinds": ["input_applied"]}, "access_detail": "none", "state_diffs": False},
    "capabilities": [],
    "native_evidence": {"enabled": False, "all": False, "kinds": []},
    "frames": {"capture": "hashes"},
    "snapshots": {"capture": "none"},
    "input_value_evidence": {"enabled": False},
    "budgets": {},
    "overflow": "fail",
}


def typed(value: Any) -> dict:
    if isinstance(value, bool):
        return {"Bool": value}
    if isinstance(value, int):
        return {"Unsigned" if value >= 0 else "Signed": value}
    if isinstance(value, str):
        return {"Text": value}
    if isinstance(value, dict):
        return {"Map": {key: typed(item) for key, item in value.items()}}
    raise TypeError(f"unsupported experiment value: {type(value).__name__}")


def validate_protocol(protocol: dict) -> None:
    if protocol["schema"] != "glass-relay.experiment" or protocol["version"] != 1:
        raise ValueError("unsupported Glass Relay experiment schema")
    trials = protocol["trials"]
    if not 1 <= len(trials) <= MAX_TRIALS:
        raise ValueError(f"protocol requires 1–{MAX_TRIALS} trials")
    if not 1 <= protocol["repetitions"] <= 8:
        raise ValueError("protocol requires 1–8 repetitions")
    if not 1 <= protocol["frame_limit"] <= 4096:
        raise ValueError("protocol requires a finite window of at most 4096 frames")
    last_release = 0
    for trial in trials:
        if not last_release < trial["press"] < trial["release"] < protocol["frame_limit"]:
            raise ValueError("trial windows must be ordered, disjoint, and preceded by baseline frames")
        last_release = trial["release"]
    for key in ("response_deadline_frames", "restoration_deadline_frames"):
        if not isinstance(protocol[key], int) or protocol[key] < 0:
            raise ValueError(f"invalid {key}")


def schedule(protocol: dict, binding: dict) -> dict:
    entries = []
    # Supply the current logical state every frame for every machine. This is
    # valid for latched keys/buttons and for frame-scoped complete gamepad masks.
    for frame in range(protocol["frame_limit"]):
        held = any(trial["press"] <= frame < trial["release"] for trial in protocol["trials"])
        action = "pressed" if held else "released"
        entries.append({"ordinal": frame, "coordinate": {"kind": "frame", "value": {"frame": frame}},
                        "input_id": binding["input_id"],
                        "payload": {"schema": binding["input_schema"], "value": typed(binding[action])}})
    return {"schema": {"id": "glassvm.input_schedule", "version": V1}, "entries": entries}


class RelayConsumer:
    """No machine/variant dispatch, event history, recorder access, or frame cache.

    Frame summaries use the declared trial windows. Input application facts are
    matched by scheduled ordinal afterward. Response metrics are withheld if
    acceptance coordinates differ, so cross-channel delivery order is irrelevant.
    """

    def __init__(self, protocol: dict, input_id: str, input_schema: dict):
        validate_protocol(protocol)
        self.protocol = {key: protocol[key] for key in PROTOCOL_KEYS}
        self.input_id = input_id
        self.input_schema = input_schema
        self.accepted: dict[int, int] = {}
        self.input_applications = 0
        self.frames = 0
        self.baseline: bytes | None = None
        self.previous: bytes | None = None
        self.baseline_changes = 0
        self.representation_schema: dict | None = None
        self.receipt: dict | None = None
        self.provenance: dict | None = None
        self.finished: dict | None = None
        self.errors: list[str] = []
        self.error_count = 0
        self.trials = [dict(spec, baseline_at_press=None, first_response=None, first_restoration=None,
                            active_digest=None, unstable_hold_frames=0, post_restoration_changes=0)
                       for spec in protocol["trials"]]

    def error(self, message: str) -> None:
        self.error_count += 1
        if len(self.errors) < MAX_STREAM_ERRORS:
            self.errors.append(message)

    def emit_batch(self, emissions: list[dict]) -> None:
        for emission in emissions:
            kind, record = emission["kind"], emission["record"]
            if kind == "run_started":
                self.provenance = {key: record.get(key) for key in (
                    "run_id", "machine_id", "prepared_run_identity", "prepared_observation_id")}
            elif kind == "frame":
                self.frame(record)
            elif kind == "event":
                self.input_applied(record)
            elif kind == "evidence_receipt":
                self.receipt = record
            elif kind == "run_finished":
                self.finished = record
            else:
                raise ValueError(f"unrequested emission channel: {kind}")

    def record_capability_outputs(self, outputs: list) -> None:
        if outputs:
            raise ValueError("Relay requested no capabilities")

    def record_capability_receipts(self, receipts: list) -> None:
        if receipts:
            raise ValueError("Relay requested no capability receipts")

    def input_applied(self, event: dict) -> None:
        if event["kind"] != "input_applied":
            raise ValueError(f"unrequested normalized kind: {event['kind']}")
        fact = event["extensions"]["glassvm.input_applied"]
        source, coordinate = fact["source"], fact["application_coordinate"]
        if source["kind"] != "scheduled" or coordinate["kind"] != "frame":
            self.error("input application has an unexpected source or coordinate")
            return
        ordinal, frame = source["ordinal"], coordinate["value"]["frame"]
        if not isinstance(ordinal, int) or not 0 <= ordinal < self.protocol["frame_limit"]:
            self.error("input application has an unexpected ordinal")
            return
        if ordinal != self.input_applications:
            self.error(f"input ordinal {ordinal} is not the next expected ordinal {self.input_applications}")
        self.input_applications += 1
        if fact["input_id"] != self.input_id or fact["input_schema"] != self.input_schema:
            self.error(f"wrong input identity/schema at ordinal {ordinal}")
        if event["frame"] != frame:
            self.error(f"inconsistent event/application coordinates at ordinal {ordinal}")
        if frame != ordinal:
            self.error(f"input ordinal {ordinal} applied at unexpected frame {frame}")
        if any(ordinal in (trial["press"], trial["release"]) for trial in self.trials):
            self.accepted[ordinal] = frame

    def frame(self, artifact: dict) -> None:
        frame = artifact["frame"]
        if frame != self.frames or frame >= self.protocol["frame_limit"]:
            raise ValueError("frame channel must contain exactly the contiguous requested frame window")
        evidence = artifact["evidence"]
        if set(evidence) != {"fingerprint"}:
            raise ValueError("Relay consumes fingerprint artifacts only")
        digest = bytes(evidence["fingerprint"]["bytes"])
        if not 1 <= len(digest) <= 128:
            raise ValueError("fingerprint is empty or exceeds consumer's fixed 128-byte limit")
        representation = artifact["representation_schema"]
        if self.representation_schema is None:
            self.representation_schema = representation
        elif representation != self.representation_schema:
            raise ValueError("fingerprint schema changed within the run")
        if self.baseline is not None and len(digest) != len(self.baseline):
            raise ValueError("fingerprint length changed within one representation schema")
        if self.baseline is None:
            self.baseline = digest
        if frame < self.trials[0]["press"]:
            self.baseline_changes += digest != self.baseline
        for index, trial in enumerate(self.trials):
            end = self.trials[index + 1]["press"] if index + 1 < len(self.trials) else self.protocol["frame_limit"]
            if trial["press"] <= frame < trial["release"]:
                if frame == trial["press"]:
                    trial["baseline_at_press"] = self.previous == self.baseline
                if trial["first_response"] is None and digest != self.baseline:
                    trial["first_response"], trial["active_digest"] = frame, digest
                elif trial["first_response"] is not None and digest != trial["active_digest"]:
                    trial["unstable_hold_frames"] += 1
                break
            if trial["release"] <= frame < end:
                if trial["first_restoration"] is None and digest == self.baseline:
                    trial["first_restoration"] = frame
                elif trial["first_restoration"] is not None and digest != self.baseline:
                    trial["post_restoration_changes"] += 1
                break
        self.previous = digest
        self.frames += 1

    def finish(self, result: dict) -> dict:
        evidence = result["evidence_fulfillment"]
        adequate = (self.frames == self.protocol["frame_limit"] and self.receipt == evidence
                    and evidence["status"] == "complete" and not self.baseline_changes
                    and self.provenance is not None and self.finished is not None
                    and self.provenance.get("prepared_run_identity") is not None
                    and self.provenance.get("prepared_observation_id") is not None
                    and result["machine_execution"]["status"] == "completed"
                    and self.input_applications == self.protocol["frame_limit"]
                    and result["python_sink_delivery"]["status"] == "complete" and not self.error_count)
        rows = []
        for index, trial in enumerate(self.trials):
            press, release = self.accepted.get(trial["press"]), self.accepted.get(trial["release"])
            coordinates_match = press == trial["press"] and release == trial["release"]
            ready = adequate and coordinates_match
            precondition = trial["baseline_at_press"] is True
            response = trial["first_response"] if ready and precondition else None
            restoration = trial["first_restoration"] if ready and response is not None else None
            latency = response - press if response is not None else None
            restore_latency = restoration - release if restoration is not None else None
            failures = []
            if ready:
                if not precondition:
                    failures.append("not_at_baseline_before_press")
                elif latency is None:
                    failures.append("no_visible_response")
                elif latency > self.protocol["response_deadline_frames"]:
                    failures.append("response_deadline")
                if trial["first_restoration"] is None:
                    failures.append("not_restored")
                elif restore_latency is not None and restore_latency > self.protocol["restoration_deadline_frames"]:
                    failures.append("restoration_deadline")
                if trial["unstable_hold_frames"]:
                    failures.append("unstable_while_held")
                if trial["post_restoration_changes"]:
                    failures.append("unstable_after_restoration")
            rows.append({"trial": index + 1, "requested_press": trial["press"], "requested_release": trial["release"],
                         "accepted_press": press, "accepted_release": release, "baseline_at_press": precondition,
                         "response_frame": response, "response_delay_frames": latency,
                         "restoration_frame": restoration, "restoration_delay_frames": restore_latency,
                         "unstable_hold_frames": trial["unstable_hold_frames"],
                         "post_restoration_changes": trial["post_restoration_changes"],
                         "status": "inconclusive" if not ready else "fail" if failures else "pass", "failures": failures})
        status = "inconclusive" if any(row["status"] == "inconclusive" for row in rows) else "fail" if any(row["failures"] for row in rows) else "pass"
        return {"status": status, "frames_observed": self.frames, "input_applications": self.input_applications,
                "baseline_changes": self.baseline_changes, "stream_error_count": self.error_count,
                "stream_errors": self.errors, "frame_representation_schema": self.representation_schema,
                "provenance": self.provenance, "trials": rows,
                "outcomes": {key: result[key] for key in (
                    "machine_execution", "evidence_fulfillment", "python_sink_delivery", "recorder_finalization", "publication")}}


def git_metadata(root: Path) -> dict:
    def git(*args: str) -> str:
        return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()
    return {"revision": git("rev-parse", "HEAD"), "branch": git("branch", "--show-current"),
            "dirty": bool(git("status", "--porcelain"))}


def render_svg(runs: list[dict], protocol: dict) -> str:
    """Intervals are reconstructed from bounded trial summaries, not a trace."""
    rows = [run for run in runs if run["repetition"] == 1]
    height, x0, plot_width = 170 + 100 * len(rows), 240, 700
    x = lambda frame: x0 + plot_width * frame / protocol["frame_limit"]
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 {height}" role="img" aria-labelledby="title desc">',
             '<title id="title">Glass Relay — one protocol, three machines</title>',
             '<desc id="desc">Requested holds and first-response/restoration spans; delays use completed frame coordinates. These are boundary summaries, not reconstructed per-frame traces. Hatching flags a failed baseline precondition or unstable hold. No raw frame hashes are compared between machines.</desc>',
             f'<rect width="1000" height="{height}" rx="24" fill="#f4f2e9"/>',
             '<text x="35" y="48" font-family="sans-serif" font-size="29" font-weight="700" fill="#173b38">GLASS RELAY</text>',
             '<text x="35" y="75" font-family="sans-serif" font-size="13" fill="#526a62">One tester · three machines · press → visible response → release → baseline</text>',
             '<text x="35" y="101" font-family="sans-serif" font-size="11" fill="#526a62">Grey: requested hold    Teal: response-to-return span    Coral: requirement failed    Hatch: unstable / unarmed</text>',
             '<defs><pattern id="unarmed" width="6" height="6" patternUnits="userSpaceOnUse"><path d="M0 0 L6 6" stroke="#ce7866" stroke-width="2"/></pattern></defs>']
    for index, run in enumerate(rows):
        y = 143 + 100 * index
        color = "#287e70" if run["status"] == "pass" else "#c36c59" if run["status"] == "fail" else "#897e60"
        parts.extend([f'<text x="35" y="{y}" font-family="sans-serif" font-size="14" font-weight="700" fill="#173b38">{html.escape(run["machine_id"])} / {html.escape(run["variant"])}</text>',
                      f'<text x="35" y="{y + 22}" font-family="sans-serif" font-size="12" fill="{color}">{run["status"].upper()} · evidence {run["outcomes"]["evidence_fulfillment"]["status"]}</text>',
                      f'<path d="M{x0} {y + 47} H{x0 + plot_width}" stroke="#d0d8d0"/>'])
        for trial in run["trials"]:
            press, release = trial["requested_press"], trial["requested_release"]
            parts.append(f'<rect x="{x(press):.1f}" y="{y - 9}" width="{x(release) - x(press):.1f}" height="8" rx="2" fill="#aab6ad"/>')
            if trial["response_frame"] is not None:
                end = trial["restoration_frame"] if trial["restoration_frame"] is not None else protocol["frame_limit"]
                fill = "url(#unarmed)" if trial["unstable_hold_frames"] else color
                parts.append(f'<rect x="{x(trial["response_frame"]):.1f}" y="{y + 9}" width="{x(end) - x(trial["response_frame"]):.1f}" height="15" rx="3" fill="{fill}"/>')
            elif not trial["baseline_at_press"]:
                parts.append(f'<rect x="{x(press):.1f}" y="{y + 9}" width="{x(release) - x(press):.1f}" height="15" fill="url(#unarmed)"/>')
            label = "unarmed" if not trial["baseline_at_press"] else f'{trial["response_delay_frames"] if trial["response_delay_frames"] is not None else "?"} / {trial["restoration_delay_frames"] if trial["restoration_delay_frames"] is not None else "?"}'
            parts.append(f'<text x="{x(press):.1f}" y="{y + 40}" font-family="sans-serif" font-size="10" fill="#526a62">{label}</text>')
    bottom = height - 40
    for frame in range(0, protocol["frame_limit"] + 1, 20):
        parts.append(f'<text x="{x(frame):.1f}" y="{bottom}" text-anchor="middle" font-family="sans-serif" font-size="11" fill="#526a62">{frame}</text>')
    parts.append(f'<text x="{x0}" y="{height - 15}" font-family="sans-serif" font-size="10" fill="#526a62">Frame coordinate · annotations: response / restoration delay in frames; ? = absent</text></svg>')
    return "\n".join(parts) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--machines-root", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--experiment", type=Path, default=EXPERIMENT)
    args = parser.parse_args()
    protocol = json.loads(args.experiment.read_text())
    validate_protocol(protocol)
    root = args.machines_root.resolve()
    subprocess.run([sys.executable, str(root / "conformance/paper/glass_relay/build_cartridges.py"), "--check"], check=True)
    artifacts = json.loads((root / "conformance/paper/glass_relay/artifacts.json").read_text())
    import glassvm
    runtime = glassvm.Runtime.discover()
    providers = json.loads(runtime.bundles())
    if not {binding["machine_id"] for binding in protocol["machines"]} <= {item["machine_id"] for item in providers}:
        raise RuntimeError("install the experiment's bundle providers before running Relay")
    args.output_dir.mkdir(parents=True, exist_ok=True)
    runs = []
    for binding in protocol["machines"]:
        for artifact in artifacts["artifacts"]:
            if artifact["machine_id"] != binding["machine_id"]:
                continue
            data = (root / artifact["path"]).read_bytes()
            if hashlib.sha256(data).hexdigest() != artifact["sha256"]:
                raise ValueError(f"artifact differs from manifest: {artifact['path']}")
            for repetition in range(1, protocol["repetitions"] + 1):
                prepared = runtime.prepare(binding["machine_id"], data,
                    configuration={"schema": binding["configuration_schema"], "value": typed(binding["configuration"])},
                    input_schedule=schedule(protocol, binding), observation=OBSERVATION,
                    execution_controls={"frame_limit": protocol["frame_limit"], "step_limit": None, "bundle_limits": []})
                consumer = RelayConsumer(protocol, binding["input_id"], binding["input_schema"])
                outcome = consumer.finish(prepared.execute(sinks=[consumer]))
                row = dict(outcome, machine_id=binding["machine_id"], variant=artifact["variant"],
                           repetition=repetition, artifact=artifact)
                runs.append(row)
    comparisons = []
    for binding in protocol["machines"]:
        for variant in artifacts["variants"]:
            group = [run for run in runs if run["machine_id"] == binding["machine_id"] and run["variant"] == variant]
            comparisons.append({"machine_id": binding["machine_id"], "variant": variant,
                                "same_trial_results": all(row["trials"] == group[0]["trials"] for row in group),
                                "same_prepared_run_identity": all(row["provenance"] and group[0]["provenance"] and row["provenance"]["prepared_run_identity"] is not None and row["provenance"]["prepared_run_identity"] == group[0]["provenance"]["prepared_run_identity"] for row in group),
                                "same_prepared_observation_identity": all(row["provenance"] and group[0]["provenance"] and row["provenance"]["prepared_observation_id"] is not None and row["provenance"]["prepared_observation_id"] == group[0]["provenance"]["prepared_observation_id"] for row in group)})
    cross_machine = []
    for variant in artifacts["variants"]:
        group = [run for run in runs if run["variant"] == variant and run["repetition"] == 1]
        cross_machine.append({"variant": variant, "machines": [run["machine_id"] for run in group],
                              "same_protocol_measurements": all(run["trials"] == group[0]["trials"] for run in group)})
    controls = []
    for run in runs:
        if run["variant"] == "reference":
            expected = run["status"] == "pass" and all(trial["response_delay_frames"] == 0 and trial["restoration_delay_frames"] == 0 for trial in run["trials"])
        elif run["variant"] == "delayed":
            expected = run["status"] == "fail" and all(trial["response_delay_frames"] == 3 and trial["restoration_delay_frames"] == 0 and trial["failures"] == ["response_deadline"] for trial in run["trials"])
        else:
            expected = run["status"] == "fail" and all("not_restored" in trial["failures"] for trial in run["trials"]) and run["trials"][0]["response_delay_frames"] == 0 and all(trial["response_delay_frames"] is None for trial in run["trials"][1:])
        controls.append(expected)
    report = {"schema": "glass-relay.report", "version": 1, "created_at": datetime.now(UTC).isoformat(),
              "python": platform.python_version(),
              "packages": {name: importlib.metadata.version(name) for name in ("glassvm", "glassvm-chip8", "glassvm-pico8", "glassvm-tic80")},
              "sources": {"glassvm": git_metadata(Path(__file__).resolve().parents[3]), "machines": git_metadata(root)},
              "protocol": protocol, "observation": OBSERVATION, "providers": providers,
              "runs": runs, "repeatability": comparisons, "cross_machine_comparisons": cross_machine,
              "control_matrix_passed": all(controls) and all(item["same_protocol_measurements"] for item in cross_machine) and all(item["same_trial_results"] and item["same_prepared_run_identity"] and item["same_prepared_observation_identity"] for item in comparisons)}
    # Prepared identities contain canonical byte arrays. Compact JSON preserves
    # their exact supplied values without expanding each byte to a separate line.
    (args.output_dir / "report.json").write_text(json.dumps(report, sort_keys=True, separators=(",", ":")) + "\n")
    (args.output_dir / "comparison.svg").write_text(render_svg(runs, protocol))
    lines = ["# Glass Relay", "", "One generic bounded Python consumer evaluates the same press/hold/release protocol on three machines.", "",
             "The comparator receives input bindings and protocol coordinates, never machine/variant-specific evaluation logic.",
             f"Packages: {report['packages']}. CPython {report['python']}. {protocol['repetitions']} repetitions per cell.",
             "This focused validation records working-tree revisions/dirty state; it is not a final sealed paper regeneration.",
             "The same explicit current-state input is scheduled at every frame for all machines, covering both latched keys and frame-scoped gamepad masks.", "",
             "| Machine | Cartridge | Protocol | Response delays | Restoration delays | Repeatable |",
             "| --- | --- | --- | --- | --- | --- |"]
    for run in (row for row in runs if row["repetition"] == 1):
        repeat = next(item for item in comparisons if item["machine_id"] == run["machine_id"] and item["variant"] == run["variant"])
        response = ", ".join(str(trial["response_delay_frames"]) if trial["response_delay_frames"] is not None else "—" for trial in run["trials"])
        restoration = ", ".join(str(trial["restoration_delay_frames"]) if trial["restoration_delay_frames"] is not None else "—" for trial in run["trials"])
        lines.append(f"| {run['machine_id']} | {run['variant']} | {run['status']} | {response} | {restoration} | {repeat['same_trial_results']} |")
    lines += ["", f"Controlled-variant matrix: {'PASS' if report['control_matrix_passed'] else 'FAIL'}. The common protocol measurements match across all three machines for each variant.",
              "These observations cover this task/configuration only; they are not general machine equivalence or execution-speed results.", "",
              "Zero delay means a visible response at completion of the input-acceptance frame. All timings use declared semantic frames under the recorded configurations.",
              "— means absent or unmeasurable. A stuck prior trial leaves later trials unarmed; its persistent marker is not misattributed to a new input.",
              "Evidence/consumer failure or unexpected acceptance coordinates make the trial inconclusive. Recorder and publication outcomes remain independent.", "",
              "[Comparison diagram](comparison.svg) · [Structured results and provenance](report.json)", ""]
    (args.output_dir / "SUMMARY.md").write_text("\n".join(lines))
    print(args.output_dir / "SUMMARY.md")
    return 0 if report["control_matrix_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
