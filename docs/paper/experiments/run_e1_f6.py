#!/usr/bin/env python3
"""F6: exhaust normalized event count under both overflow policies."""
import argparse, hashlib, importlib.metadata, importlib.util, json, platform, subprocess, sys, tempfile, tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];MACHINES=ROOT.parent/'glassvm-machines';HELPER=MACHINES/'conformance/paper/ce1/run_ce1.py';OUT=ROOT/'docs/paper/experiments/measurements/e1-f6-local-repair'
def worker(machine,policy,rep):
    spec=importlib.util.spec_from_file_location('ce1_helpers',HELPER);h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
    base=MACHINES/machine/'fixtures/paper'
    if machine!='chip8':base/='ce1'
    c=tomllib.loads((base/'cohort.toml').read_text());w=c['workload'][0];artifact=(MACHINES/machine/'fixtures'/w['artifact_path']).read_bytes();digest=hashlib.sha256(artifact).hexdigest()
    if digest!=w['sha256']:raise ValueError('fixture digest mismatch')
    obs=h.observation_request({'normalized_events':'none','frames':'none'},c);obs['normalized_events']['events']={'kinds':['frame_completed']};obs['budgets']={'max_normalized_events':1};obs['overflow']=policy
    config=h.machine_configuration(machine,c['configuration']);schedule={'schema':h.INPUT_SCHEDULE_SCHEMA,'entries':[]};controls={'frame_limit':2,'step_limit':None,'bundle_limits':[]}
    import glassvm
    prepared=glassvm.Runtime.discover().prepare(machine,artifact,configuration=config,input_schedule=schedule,execution_controls=controls,observation=obs)
    with tempfile.TemporaryDirectory(prefix='glassvm-e1-f6-') as d:
        result=prepared.execute(file_recorder=glassvm.FileRecorder(d+'/published'))
        paths=[str(x.relative_to(d)) for x in Path(d).rglob('*') if x.is_file()]
    return {'case':'F6','machine':machine,'policy':policy,'repetition':rep,'artifact':{'path':w['artifact_path'],'sha256':digest},'request':{'configuration':config,'schedule':schedule,'controls':controls,'observation':obs},'preparation':'accepted','result':result,'file_paths_before_cleanup':paths}
def main():
    p=argparse.ArgumentParser();p.add_argument('--worker',choices=['chip8','pico8','tic80']);p.add_argument('--policy');p.add_argument('--repetition',type=int);p.add_argument('--output-dir',type=Path,default=OUT);a=p.parse_args()
    if a.worker:print(json.dumps(worker(a.worker,a.policy,a.repetition)));return
    if a.output_dir.exists() and any(a.output_dir.iterdir()):raise SystemExit('output directory not empty')
    a.output_dir.mkdir(parents=True,exist_ok=True);rows=[]
    for machine in ('chip8','pico8','tic80'):
        for policy in ('fail','allow_incomplete'):
            for rep in range(1,4):
                proc=subprocess.run([sys.executable,'-I',str(Path(__file__).resolve()),'--worker',machine,'--policy',policy,'--repetition',str(rep)],text=True,capture_output=True)
                row=json.loads(proc.stdout) if proc.returncode==0 else {'machine':machine,'policy':policy,'repetition':rep,'worker_failure':{'returncode':proc.returncode,'stdout':proc.stdout,'stderr':proc.stderr}}
                (a.output_dir/f'{machine}-{policy}-r{rep}.json').write_text(json.dumps(row,indent=2,sort_keys=True)+'\n');rows.append(row)
                result=row.get('result',{});evidence=result.get('evidence_fulfillment') or {};channel=next((x for x in evidence.get('channels',[]) if x.get('channel')=='normalized_events'),{})
                expected=('interrupted_by_evidence_failure','failed','not_finalized','not_attempted','failed') if policy=='fail' else ('completed','incomplete','finalized','published','truncated')
                actual=(result.get('machine_execution',{}).get('status'),evidence.get('status'),result.get('recorder_finalization',{}).get('status'),result.get('publication',{}).get('status'),channel.get('status'))
                ok=actual==expected and channel.get('item_count')==1
                print(machine,policy,rep,'pass' if ok else f'unexpected {actual}',flush=True)
                if not ok:
                    (a.output_dir/'partial.json').write_text(json.dumps(rows,indent=2)+'\n');raise SystemExit('unexpected F6 outcome')
    rev=lambda root:subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip();wheels=Path('/tmp/glassvm-f1-local-wheels')
    manifest={'case':'F6','source':{'glassvm':rev(ROOT),'machines':rev(MACHINES)},'packages':{x:importlib.metadata.version(x) for x in ('glassvm','glassvm-chip8','glassvm-pico8','glassvm-tic80')},'wheel_sha256':{x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in wheels.glob('*.whl')},'host':platform.platform(),'python':sys.version,'runner_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'trials':len(rows),'rows':rows}
    (a.output_dir/'manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
if __name__=='__main__':main()
