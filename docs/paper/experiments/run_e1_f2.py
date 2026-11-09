#!/usr/bin/env python3
"""F2 optional unavailable capability: isolated public Python executions."""
import argparse, hashlib, importlib.metadata, importlib.util, json, platform, subprocess, sys, tempfile, tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
MACHINES=ROOT.parent/'glassvm-machines'
HELPER=MACHINES/'conformance/paper/ce1/run_ce1.py'
CAPS={'chip8':'chip8.input_summary','pico8':'pico8.input_summary','tic80':'tic80.input_summary'}
OUT=ROOT/'docs/paper/experiments/measurements/e1-f2-local-repair'

def worker(machine,rep):
    spec=importlib.util.spec_from_file_location('ce1_helpers',HELPER); h=importlib.util.module_from_spec(spec); spec.loader.exec_module(h)
    base=MACHINES/machine/'fixtures/paper'
    if machine!='chip8': base/='ce1'
    c=tomllib.loads((base/'cohort.toml').read_text()); w=c['workload'][0]
    artifact=(MACHINES/machine/'fixtures'/w['artifact_path']).read_bytes(); digest=hashlib.sha256(artifact).hexdigest()
    if digest!=w['sha256']: raise ValueError('fixture digest mismatch')
    obs=h.observation_request({'normalized_events':'none','frames':'none'},c)
    obs['capabilities']=[{'id':CAPS[machine],'required':False,'parameters':{}}]
    config=h.machine_configuration(machine,c['configuration']); schedule={'schema':h.INPUT_SCHEDULE_SCHEMA,'entries':[]}; controls={'frame_limit':2,'step_limit':None,'bundle_limits':[]}
    import glassvm
    prepared=glassvm.Runtime.discover().prepare(machine,artifact,configuration=config,input_schedule=schedule,execution_controls=controls,observation=obs)
    with tempfile.TemporaryDirectory(prefix='glassvm-e1-f2-') as d:
        result=prepared.execute(file_recorder=glassvm.FileRecorder(d+'/published'))
    return {'machine':machine,'case':'F2','repetition':rep,'artifact':{'path':w['artifact_path'],'sha256':digest},'request':{'configuration':config,'schedule':schedule,'controls':controls,'observation':obs},'preparation':{'status':'accepted','prospective_details':'opaque in Python facade'},'result':result}

def main():
    p=argparse.ArgumentParser();p.add_argument('--worker',choices=list(CAPS));p.add_argument('--repetition',type=int);p.add_argument('--output-dir',type=Path,default=OUT);a=p.parse_args()
    if a.worker:
        print(json.dumps(worker(a.worker,a.repetition)));return
    if a.output_dir.exists() and any(a.output_dir.iterdir()): raise SystemExit('output directory not empty')
    a.output_dir.mkdir(parents=True,exist_ok=True);rows=[]
    for machine in CAPS:
        for rep in range(1,4):
            cmd=[sys.executable,'-I',str(Path(__file__).resolve()),'--worker',machine,'--repetition',str(rep)]
            p=subprocess.run(cmd,text=True,capture_output=True)
            row=json.loads(p.stdout) if p.returncode==0 else {'machine':machine,'repetition':rep,'worker_failure':{'returncode':p.returncode,'stdout':p.stdout,'stderr':p.stderr}}
            (a.output_dir/f'{machine}-r{rep}.json').write_text(json.dumps(row,indent=2,sort_keys=True)+'\n'); rows.append(row)
            result=row.get('result',{}); evidence=result.get('evidence_fulfillment') or {}; caps=evidence.get('capabilities',[])
            ok=(result.get('machine_execution',{}).get('status')=='completed' and result.get('recorder_finalization',{}).get('status')=='finalized' and result.get('publication',{}).get('status')=='published' and any(x.get('id')==CAPS[machine] and x.get('status')=='unavailable' and 'input-value evidence channel' in (x.get('message') or '') for x in caps))
            print(machine,rep,'pass' if ok else 'unexpected',flush=True)
            if not ok:
                (a.output_dir/'partial.json').write_text(json.dumps(rows,indent=2)+'\n');raise SystemExit(f'unexpected outcome for {machine} repetition {rep}')
    rev=lambda root:subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()
    wheels=Path('/tmp/glassvm-f1-local-wheels')
    manifest={'case':'F2','source':{'glassvm':rev(ROOT),'machines':rev(MACHINES)},'packages':{x:importlib.metadata.version(x) for x in ('glassvm','glassvm-chip8','glassvm-pico8','glassvm-tic80')},'wheel_sha256':{x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in wheels.glob('*.whl')},'host':platform.platform(),'python':sys.version,'runner_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'trials':len(rows),'rows':rows}
    (a.output_dir/'manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
if __name__=='__main__':main()
