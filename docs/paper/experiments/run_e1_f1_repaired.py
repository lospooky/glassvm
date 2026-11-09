#!/usr/bin/env python3
"""Rerun F1 against locally built machine wheels, with fresh worker processes."""
import argparse
import hashlib
import importlib.metadata
import importlib.util
import json
import platform
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MACHINES = ROOT.parent / 'glassvm-machines'
HELPER = MACHINES / 'conformance/paper/ce1/run_ce1.py'
OUT = ROOT / 'docs/paper/experiments/measurements/e1-f1-local-repair'
NATIVE = {'chip8':'chip8.execution_summary','pico8':'pico8.execution_summary','tic80':'tic80.execution_summary'}


def worker(machine, case, repetition):
    spec=importlib.util.spec_from_file_location('ce1_helpers',HELPER)
    h=importlib.util.module_from_spec(spec); spec.loader.exec_module(h)
    base=MACHINES / machine / 'fixtures/paper'
    if machine != 'chip8': base /= 'ce1'
    cohort=tomllib.loads((base/'cohort.toml').read_text())
    workload=cohort['workload'][0]
    artifact=(MACHINES/machine/'fixtures'/workload['artifact_path']).read_bytes()
    digest=hashlib.sha256(artifact).hexdigest()
    if digest != workload['sha256']: raise ValueError('fixture digest mismatch')
    obs=h.observation_request({'normalized_events':'none','frames':'none'},cohort)
    if case == 'capability':
        obs['capabilities']=[{'id':'glassvm.paper.unsupported-capability','required':True,'parameters':{}}]
    elif case == 'normalized':
        obs['normalized_events']['events']={'kinds':[{'extension':'glassvm.paper.unsupported-event'}]}
    elif case == 'native':
        obs['native_evidence']={'enabled':True,'all':False,'kinds':['glassvm.paper.unsupported-native']}
    else:
        obs['normalized_events']['events']={'kinds':['frame_completed']}
        obs['native_evidence']={'enabled':True,'all':False,'kinds':[NATIVE[machine]]}
    config=h.machine_configuration(machine,cohort['configuration'])
    schedule={'schema':h.INPUT_SCHEDULE_SCHEMA,'entries':[]}
    controls={'frame_limit':600,'step_limit':None,'bundle_limits':[]}
    import glassvm
    try:
        glassvm.Runtime.discover().prepare(machine,artifact,configuration=config,input_schedule=schedule,execution_controls=controls,observation=obs)
        outcome={'status':'accepted'}
    except Exception as e:
        outcome={'status':'rejected','error_type':type(e).__name__,'message':str(e)}
    return {'machine':machine,'case':case,'repetition':repetition,'artifact':{'path':workload['artifact_path'],'sha256':digest},'configuration':config,'schedule':schedule,'controls':controls,'observation':obs,'preparation':outcome,'execution_invoked':False}


def main():
    p=argparse.ArgumentParser(); p.add_argument('--worker',choices=['chip8','pico8','tic80']); p.add_argument('--case'); p.add_argument('--repetition',type=int); p.add_argument('--output-dir',type=Path,default=OUT); a=p.parse_args()
    if a.worker:
        print(json.dumps(worker(a.worker,a.case,a.repetition))); return
    if a.output_dir.exists() and any(a.output_dir.iterdir()): raise SystemExit('output directory is not empty')
    a.output_dir.mkdir(parents=True,exist_ok=True)
    rows=[]
    for machine in ('chip8','pico8','tic80'):
        for case in ('capability','normalized','native','supported'):
            for rep in range(1, 2 if case=='supported' else 4):
                proc=subprocess.run([sys.executable,'-I',str(Path(__file__).resolve()),'--worker',machine,'--case',case,'--repetition',str(rep)],text=True,capture_output=True)
                row=json.loads(proc.stdout) if proc.returncode==0 else {'machine':machine,'case':case,'repetition':rep,'worker_failure':{'code':proc.returncode,'stderr':proc.stderr,'stdout':proc.stdout}}
                (a.output_dir/f'{machine}-{case}-{rep}.json').write_text(json.dumps(row,indent=2,sort_keys=True)+'\n')
                rows.append(row)
                outcome=row.get('preparation',{}); expected='accepted' if case=='supported' else 'rejected'
                if outcome.get('status')!=expected or (case!='supported' and 'unsupported' not in outcome.get('message','')):
                    (a.output_dir/'partial.json').write_text(json.dumps(rows,indent=2)+'\n')
                    raise SystemExit(f'unexpected {machine} {case} {rep}: {row}')
                print(machine,case,rep,outcome['status'],flush=True)
    packages={name:importlib.metadata.version(name) for name in ('glassvm','glassvm-chip8','glassvm-pico8','glassvm-tic80')}
    rev=lambda root: subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()
    wheel_dir=Path('/tmp/glassvm-f1-local-wheels')
    manifest={'description':'F1 repaired local-source wheel trials; no execution invoked','glassvm_revision':rev(ROOT),'machines_revision':rev(MACHINES),'packages':packages,'wheels':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(wheel_dir.glob('*.whl'))},'host':platform.platform(),'cpu':next((line.split(':',1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name')),None),'python':sys.version,'runner_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'trials':len(rows),'rows':rows}
    (a.output_dir/'manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')

if __name__=='__main__': main()
