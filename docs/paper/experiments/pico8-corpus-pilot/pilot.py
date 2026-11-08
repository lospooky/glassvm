#!/usr/bin/env python3
"""Screen upstream carts in isolated processes; exploratory, not a paper benchmark."""
import concurrent.futures
import hashlib
import importlib.metadata
import io
import json
import os
import platform
import re
import subprocess
import sys
import time
import urllib.request
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from run_trial import machine_configuration

URL = 'https://downloads.seans.site/tweetcarts-2017/tweetcarts-2017-source.zip'
ARCHIVE_SHA256 = 'e45625ca2dc880476fb2dba0a8cbc539f5cd7108f231705b4c71f715373e5ff7'
TIMEOUT = 4
FRAMES = 10
OBS = {'schema_version': {'major': 2, 'minor': 1, 'patch': 0}, 'guarantee': 'summary', 'normalized_events': {'events': 'none', 'access_detail': 'none', 'state_diffs': False}, 'capabilities': [], 'native_evidence': {'enabled': False, 'all': False, 'kinds': []}, 'frames': {'capture': 'hashes'}, 'snapshots': {'capture': 'none'}, 'input_value_evidence': {'enabled': False}, 'budgets': {}, 'overflow': 'fail'}
SCHEDULE = {'schema': {'id': 'glassvm.input_schedule', 'version': {'major': 1, 'minor': 0, 'patch': 0}}, 'entries': []}
CONFIG = machine_configuration('pico8', {'cycles_per_frame': 1, 'instruction_budget': 500000, 'machine_seed': 0})

class Sink:
    def __init__(self): self.frames = 0; self.fingerprints = set()
    def emit_batch(self, batch):
        for item in batch:
            if item['kind'] == 'frame':
                self.frames += 1
                self.fingerprints.add(json.dumps(item['record'].get('evidence'), sort_keys=True))
    def record_capability_outputs(self, values): pass
    def record_capability_receipts(self, values): pass

def child():
    import glassvm
    artifact = sys.stdin.buffer.read()
    phase = 'preparation'
    try:
        prepared = glassvm.Runtime.discover().prepare('pico8', artifact, configuration=CONFIG, input_schedule=SCHEDULE, observation=OBS, execution_controls={'frame_limit': FRAMES, 'step_limit': None, 'bundle_limits': []})
        phase = 'execution'
        sink = Sink()
        result = prepared.execute(sinks=[sink])
        print(json.dumps({'status': 'returned', 'frames_received': sink.frames, 'distinct_fingerprints': len(sink.fingerprints), 'machine_execution': result.get('machine_execution'), 'evidence_fulfillment': result.get('evidence_fulfillment'), 'python_sink_delivery': result.get('python_sink_delivery')}))
    except Exception as error:
        print(json.dumps({'status': 'exception', 'phase': phase, 'type': type(error).__name__, 'message': str(error)}))

def main():
    archive = urllib.request.urlopen(URL, timeout=30).read()
    digest = hashlib.sha256(archive).hexdigest()
    if digest != ARCHIVE_SHA256:
        raise ValueError(f'upstream archive changed: expected {ARCHIVE_SHA256}, got {digest}')
    print('archive sha256', digest, flush=True)
    z = zipfile.ZipFile(io.BytesIO(archive))
    names = [f'tweetcart{i}.p8' for i in range(51,81)]
    def trial(name):
        raw = z.read(name)
        scan = re.search(rb'\b(btnp?|stat)\s*\(', raw) is not None
        row = {'name': name, 'sha256': hashlib.sha256(raw).hexdigest(), 'bytes': len(raw), 'direct_input_or_stat_call_scan': scan}
        start = time.monotonic()
        try:
            p = subprocess.run([sys.executable, __file__, '--child'], input=raw, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=TIMEOUT)
            row.update({'exit_code': p.returncode, 'stderr': p.stderr.decode('utf8', 'replace')[-3000:]})
            try: row['result'] = json.loads(p.stdout)
            except Exception: row['result'] = {'status': 'invalid_output', 'stdout': p.stdout.decode('utf8','replace')[-3000:]}
        except subprocess.TimeoutExpired:
            row['result'] = {'status': 'timeout', 'timeout_seconds': TIMEOUT}
        row['elapsed_seconds'] = round(time.monotonic() - start, 3)
        print(name, row['result'].get('status'), flush=True)
        return row
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        rows = list(pool.map(trial, names))
    report = {'source_url': URL, 'source_archive_sha256': digest, 'source_entry_count': len(z.namelist()), 'selection': 'consecutive IDs 51 through 80, chosen before testing', 'run': {'frames': FRAMES, 'timeout_seconds': TIMEOUT, 'workers': 3, 'configuration': CONFIG, 'input_schedule': SCHEDULE, 'observation': OBS, 'python': platform.python_version(), 'platform': platform.platform(), 'glassvm': importlib.metadata.version('glassvm'), 'glassvm_pico8': importlib.metadata.version('glassvm-pico8')}, 'trials': rows}
    (HERE / 'results.json').write_text(json.dumps(report, indent=2)+'\n')
if __name__ == '__main__':
    child() if '--child' in sys.argv else main()
