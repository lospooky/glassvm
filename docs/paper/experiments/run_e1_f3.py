#!/usr/bin/env python3
"""F3 Python consumer failures at each public callback boundary."""
import argparse, hashlib, importlib.metadata, importlib.util, json, platform, subprocess, sys, tempfile, tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]; MACHINES=ROOT.parent/'glassvm-machines'
HELPER=MACHINES/'conformance/paper/ce1/run_ce1.py'; OUT=ROOT/'docs/paper/experiments/measurements/e1-f3-local-repair'
CAPS={'chip8':'chip8.visual_trajectory_motifs','pico8':'pico8.visual.motifs','tic80':'tic80.visual.motifs'}
class FailingConsumer:
    def __init__(self,trigger): self.trigger=trigger; self.calls=[]
    def emit_batch(self,values):
        if any(value.get('kind')=='event' for value in values):
            self.calls.append('event_batch')
            if self.trigger=='event_batch': raise RuntimeError('E1 F3 event_batch')
    def record_capability_outputs(self,values):
        if values:
            self.calls.append('capability_outputs')
            if self.trigger=='capability_outputs': raise RuntimeError('E1 F3 capability_outputs')
    def record_capability_receipts(self,values):
        if values:
            self.calls.append('capability_receipts')
            if self.trigger=='capability_receipts': raise RuntimeError('E1 F3 capability_receipts')

def worker(machine,trigger,rep):
    spec=importlib.util.spec_from_file_location('ce1_helpers',HELPER);h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
    base=MACHINES/machine/'fixtures/paper'
    if machine!='chip8':base/='ce1'
    c=tomllib.loads((base/'cohort.toml').read_text());w=c['workload'][0]
    artifact=(MACHINES/machine/'fixtures'/w['artifact_path']).read_bytes(); digest=hashlib.sha256(artifact).hexdigest()
    if digest!=w['sha256']:raise ValueError('fixture digest mismatch')
    obs=h.observation_request({'normalized_events':'selected','frames':'hashes'},c);obs['capabilities']=[{'id':CAPS[machine],'required':True,'parameters':{}}]
    config=h.machine_configuration(machine,c['configuration']);schedule={'schema':h.INPUT_SCHEDULE_SCHEMA,'entries':[]};controls={'frame_limit':2,'step_limit':None,'bundle_limits':[]}
    import glassvm
    prepared=glassvm.Runtime.discover().prepare(machine,artifact,configuration=config,input_schedule=schedule,execution_controls=controls,observation=obs)
    sink=FailingConsumer(trigger)
    with tempfile.TemporaryDirectory(prefix='glassvm-e1-f3-') as d:
        result=prepared.execute(sinks=[sink],file_recorder=glassvm.FileRecorder(d+'/published'))
        paths=[str(p.relative_to(d)) for p in Path(d).rglob('*')]
    return {'machine':machine,'case':'F3','trigger':trigger,'repetition':rep,'artifact':{'path':w['artifact_path'],'sha256':digest},'request':{'configuration':config,'schedule':schedule,'controls':controls,'observation':obs},'callbacks_seen':sink.calls,'staging_paths_before_cleanup':paths,'result':result}

def main():
    p=argparse.ArgumentParser();p.add_argument('--worker',choices=list(CAPS));p.add_argument('--trigger');p.add_argument('--repetition',type=int);p.add_argument('--output-dir',type=Path,default=OUT);a=p.parse_args()
    if a.worker:print(json.dumps(worker(a.worker,a.trigger,a.repetition)));return
    if a.output_dir.exists() and any(a.output_dir.iterdir()):raise SystemExit('output directory not empty')
    a.output_dir.mkdir(parents=True,exist_ok=True);rows=[]
    for machine in CAPS:
        for trigger in ('event_batch','capability_outputs','capability_receipts'):
            for rep in range(1,4):
                proc=subprocess.run([sys.executable,'-I',str(Path(__file__).resolve()),'--worker',machine,'--trigger',trigger,'--repetition',str(rep)],text=True,capture_output=True)
                row=json.loads(proc.stdout) if proc.returncode==0 else {'machine':machine,'trigger':trigger,'repetition':rep,'worker_failure':{'returncode':proc.returncode,'stderr':proc.stderr,'stdout':proc.stdout}}
                (a.output_dir/f'{machine}-{trigger}-r{rep}.json').write_text(json.dumps(row,indent=2,sort_keys=True)+'\n');rows.append(row)
                r=row.get('result',{});delivery=r.get('python_sink_delivery',{});failure=delivery.get('failure') or {}; execution=r.get('machine_execution',{}).get('status')
                ok=(trigger in row.get('callbacks_seen',[]) and delivery.get('status')=='failed' and failure.get('exception_type')=='RuntimeError' and trigger in (failure.get('exception_message') or '') and execution in ('completed','interrupted_by_evidence_failure'))
                print(machine,trigger,rep,'pass' if ok else 'unexpected',flush=True)
                if not ok:
                    (a.output_dir/'partial.json').write_text(json.dumps(rows,indent=2)+'\n');raise SystemExit(f'unexpected F3 outcome: {machine} {trigger} {rep}')
    rev=lambda root:subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip(); wheels=Path('/tmp/glassvm-f1-local-wheels')
    manifest={'case':'F3','source':{'glassvm':rev(ROOT),'machines':rev(MACHINES)},'packages':{x:importlib.metadata.version(x) for x in ('glassvm','glassvm-chip8','glassvm-pico8','glassvm-tic80')},'wheel_sha256':{x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in wheels.glob('*.whl')},'host':platform.platform(),'python':sys.version,'runner_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'trials':len(rows),'rows':rows}
    (a.output_dir/'manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
if __name__=='__main__':main()
