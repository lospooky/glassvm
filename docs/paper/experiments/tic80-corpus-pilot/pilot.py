#!/usr/bin/env python3
"""Screen a fixed TIC-80 catalog candidate pool with the installed GlassVM provider.

The result is an exploratory compatibility screen, not a representative sample
or a claim of upstream TIC-80 fidelity. Downloaded third-party carts stay local
because many entries do not state a license.
"""

from __future__ import annotations

import concurrent.futures
import hashlib
import importlib.metadata
import json
import os
import platform
import re
import subprocess
import sys
import time
import urllib.request
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
CANDIDATES = json.loads((HERE / "candidate-set.json").read_text())
LOCAL_CARTS = HERE / "local-only-carts"
FRAMES = 10
TIMEOUT_SECONDS = 8
WORKERS = 6
MACHINE_CONFIGURATION = {
    "schema": {"id": "tic80.machine_configuration", "version": {"major": 1, "minor": 0, "patch": 0}},
    "value": {"Map": {
        "cycles_per_frame": {"Unsigned": 1},
        "runtime": {"Text": "lua"},
        "machine_seed": {"Unsigned": 0},
    }},
}
INPUT_SCHEDULE = {
    "schema": {"id": "glassvm.input_schedule", "version": {"major": 1, "minor": 0, "patch": 0}},
    "entries": [],
}
OBSERVATION = {
    "schema_version": {"major": 2, "minor": 1, "patch": 0},
    "guarantee": "summary",
    "normalized_events": {"events": "none", "access_detail": "none", "state_diffs": False},
    "capabilities": [],
    "native_evidence": {"enabled": False, "all": False, "kinds": []},
    "frames": {"capture": "hashes"},
    "snapshots": {"capture": "none"},
    "input_value_evidence": {"enabled": False},
    "budgets": {},
    "overflow": "fail",
}
EXECUTION_CONTROLS = {"frame_limit": FRAMES, "step_limit": None, "bundle_limits": []}
INPUT_API = re.compile(r"\b(?:btnp?|keyp?|mouse|pmem)\s*\(", re.IGNORECASE)
EXCLUDED_TITLE = re.compile(r"draconus|music demo|paranoimia|deltarune|christmas", re.IGNORECASE)


def extract_lua_source(cart: bytes) -> str:
    offset = 0
    pieces: list[str] = []
    while offset < len(cart):
        if len(cart) - offset < 4:
            break
        tag = cart[offset]
        kind = tag & 0x1F
        size = int.from_bytes(cart[offset + 1:offset + 3], "little")
        offset += 4
        if size == 0 and kind in (5, 19) and len(cart) - offset >= 65_536:
            size = 65_536
        if size > len(cart) - offset:
            break
        data = cart[offset:offset + size]
        offset += size
        if kind == 5:
            pieces.append(data.decode("latin-1", "replace"))
        elif kind == 16:
            try:
                pieces.append(zlib.decompress(data).decode("latin-1", "replace"))
            except zlib.error:
                pass
    return "\n".join(pieces)


def license_marker(source: str) -> tuple[str | None, str]:
    match = re.search(r"--\s*(?:license|licensed)\s*:\s*([^\r\n]+)", source, re.IGNORECASE)
    if not match:
        return None, "no_statement_found"
    marker = match.group(1).strip()
    if re.search(r"change this|your license|\?\?\?", marker, re.IGNORECASE):
        return marker, "template_or_ambiguous"
    return marker, "stated_in_cart_source"


def child() -> None:
    import glassvm

    artifact = sys.stdin.buffer.read()

    class FrameSink:
        def __init__(self) -> None:
            self.fingerprints: list[str] = []

        def emit_batch(self, emissions) -> None:
            for emission in emissions:
                if emission["kind"] == "frame":
                    raw = emission["record"]["evidence"]["fingerprint"]["bytes"]
                    self.fingerprints.append(bytes(raw).hex())

        def record_capability_outputs(self, _outputs) -> None:
            pass

        def record_capability_receipts(self, _receipts) -> None:
            pass

    sink = FrameSink()
    phase = "preparation"
    try:
        prepared = glassvm.Runtime.discover().prepare(
            "tic80", artifact, configuration=MACHINE_CONFIGURATION,
            input_schedule=INPUT_SCHEDULE, observation=OBSERVATION,
            execution_controls=EXECUTION_CONTROLS,
        )
        phase = "execution"
        result = prepared.execute(sinks=[sink])
        print(json.dumps({
            "status": "returned",
            "machine_execution": result["machine_execution"],
            "evidence_status": result["evidence_fulfillment"]["status"],
            "frame_count": len(sink.fingerprints),
            "distinct_fingerprints": len(set(sink.fingerprints)),
            "fingerprints": sink.fingerprints,
        }))
    except Exception as exc:
        print(json.dumps({"status": "exception", "phase": phase,
                          "error_type": type(exc).__name__, "error": str(exc)}))


def run_candidate(candidate: dict) -> dict:
    url = candidate["download_url"]
    request = urllib.request.Request(url, headers={"User-Agent": "GlassVM TIC-80 research screen"})
    started = time.monotonic()
    try:
        cart = urllib.request.urlopen(request, timeout=30).read()
        md5 = hashlib.md5(cart).hexdigest()
        sha256 = hashlib.sha256(cart).hexdigest()
        if md5 != candidate["catalog_md5"]:
            return candidate | {"download_status": "md5_mismatch", "actual_md5": md5}
        source = extract_lua_source(cart)
        code_for_scan = re.sub(r"--[^\n]*", "", source)
        marker, marker_status = license_marker(source)
        row = candidate | {
            "download_status": "verified",
            "actual_bytes": len(cart),
            "sha256": sha256,
            "license_marker": marker,
            "license_status": marker_status,
            "direct_input_api_scan": "matched" if INPUT_API.search(code_for_scan) else "no_direct_call_matched",
        }
        try:
            proc = subprocess.run(
                [sys.executable, str(Path(__file__).resolve()), "--child"],
                input=cart, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                timeout=TIMEOUT_SECONDS, cwd=ROOT, env=os.environ.copy(),
            )
            try:
                row["screen"] = json.loads(proc.stdout)
            except json.JSONDecodeError:
                row["screen"] = {"status": "invalid_child_output",
                                  "stdout_tail": proc.stdout.decode("utf-8", "replace")[-1200:]}
            row["child_stderr_tail"] = proc.stderr.decode("utf-8", "replace")[-1000:]
        except subprocess.TimeoutExpired:
            row["screen"] = {"status": "timeout", "timeout_seconds": TIMEOUT_SECONDS}
        result = row["screen"]
        row["screen_pass"] = (
            result.get("status") == "returned"
            and result.get("machine_execution", {}).get("status") == "completed"
            and result.get("evidence_status") == "complete"
            and result.get("frame_count") == FRAMES
            and result.get("distinct_fingerprints", 0) > 1
        )
        row["elapsed_seconds"] = round(time.monotonic() - started, 3)
        return row
    except Exception as exc:
        return candidate | {"download_status": "error", "download_error": str(exc)}


def main() -> None:
    LOCAL_CARTS.mkdir(parents=True, exist_ok=True)
    candidates = CANDIDATES["candidates"]
    with concurrent.futures.ThreadPoolExecutor(max_workers=WORKERS) as pool:
        trials = list(pool.map(run_candidate, candidates))

    eligible = [
        row for row in trials
        if row.get("screen_pass")
        and row.get("direct_input_api_scan") == "no_direct_call_matched"
        and not EXCLUDED_TITLE.search(row["name"])
    ]
    eligible.sort(key=lambda row: (row["name"].casefold(), row["page_url"]))
    selected = eligible[:30]

    selected_records = []
    for index, row in enumerate(selected, start=1):
        request = urllib.request.Request(row["download_url"], headers={"User-Agent": "GlassVM TIC-80 research screen"})
        cart = urllib.request.urlopen(request, timeout=30).read()
        if hashlib.md5(cart).hexdigest() != row["catalog_md5"]:
            raise RuntimeError(f"source changed while saving {row['name']}")
        filename = f"{index:02d}-{re.sub(r'[^A-Za-z0-9]+', '-', row['name']).strip('-')[:42]}.tic"
        path = LOCAL_CARTS / filename
        path.write_bytes(cart)
        selected_records.append({
            "index": index,
            "name": row["name"],
            "developer": row["developer"],
            "page_url": row["page_url"],
            "download_url": row["download_url"],
            "catalog_md5": row["catalog_md5"],
            "sha256": row["sha256"],
            "local_path": path.relative_to(HERE).as_posix(),
            "license_marker": row.get("license_marker"),
            "license_status": row.get("license_status"),
            "frame_count": row["screen"]["frame_count"],
            "distinct_fingerprints": row["screen"]["distinct_fingerprints"],
        })

    report = {
        "status": "exploratory_compatibility_screen",
        "catalog": CANDIDATES["catalog"],
        "selection_rule": "passing Lua candidates with no direct input API call matched in extracted source, after excluding obvious derivative/audio titles; sort by title then page URL and take first 30",
        "screen": {
            "frames": FRAMES,
            "minimum_distinct_frame_fingerprints": 2,
            "input_schedule": INPUT_SCHEDULE,
            "machine_configuration": MACHINE_CONFIGURATION,
            "observation": OBSERVATION,
            "execution_controls": EXECUTION_CONTROLS,
            "timeout_seconds_per_candidate": TIMEOUT_SECONDS,
            "workers": WORKERS,
            "python": platform.python_version(),
            "platform": platform.platform(),
            "glassvm": importlib.metadata.version("glassvm"),
            "glassvm_tic80": importlib.metadata.version("glassvm-tic80"),
        },
        "candidate_count": len(trials),
        "passing_count": sum(bool(row.get("screen_pass")) for row in trials),
        "no_input_scan_and_screen_count": len(eligible),
        "selected_count": len(selected_records),
        "selected": selected_records,
        "trials": trials,
    }
    (HERE / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    (HERE / "manifest.json").write_text(json.dumps({
        "schema_version": 1,
        "selection_rule": report["selection_rule"],
        "selected": selected_records,
    }, indent=2) + "\n")
    if len(selected_records) != 30:
        raise SystemExit(f"screened {len(selected_records)} eligible carts; see results.json")


if __name__ == "__main__":
    if "--child" in sys.argv:
        child()
    else:
        main()
