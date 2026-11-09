#!/usr/bin/env python3
"""F5: kill a machine process at the approved RunFinished callback barrier."""
import argparse, hashlib, importlib.metadata, importlib.util, json, platform, subprocess, sys, tempfile, time, tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]; MACHINES=ROOT.parent/'glassvm-machines'; HELPER=MACHINES/'conformance/paper/ce1/run_ce1.py'; OUT=ROOT/'docs/paper/experiments/measurements/e1-f5-local-repair'
class Barrier:
    def __init__(self,marker): self.marker=marker
    def emit_batch(self,values):
        if any(v.get('kind')=='run_finished' for v in values):
            self.marker.write_text('run_finished callback reached\n')
            while True: time.sleep(1)
    def record_capability_outputs(self,values): pass
    def record_capability_receipts(self,values): pass

def child(machine,d):
    spec=importlib.util.spec_from_file_location('ce1_helpers',HELPER);h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
    base=MACHINES/machine/'fixtures/paper'
    if machine!='chip8':base/='ce1'
    c=tomllib.loads((base/'cohort.toml').read_text());w=c['workload'][0];artifact=(MACHINES/machine/'fixtures'/w['artifact_path']).read_bytes()
    digest=hashlib.sha256(artifact).hexdigest()
    if digest!=w['sha256']: raise ValueError('fixture digest mismatch')
    obs=h.observation_request({'normalized_events':'selected','frames':'hashes'},c)
    config=h.machine_configuration(machine,c['configuration']);schedule={'schema':h.INPUT_SCHEDULE_SCHEMA,'entries':[]};controls={'frame_limit':2,'step_limit':None,'bundle_limits':[]}
    (d/'request.json').write_text(json.dumps({'artifact_path':w['artifact_path'],'artifact_sha256':digest,'configuration':config,'schedule':schedule,'controls':controls,'observation':obs},indent=2))
    import glassvm
    prepared=glassvm.Runtime.discover().prepare(machine,artifact,configuration=config,input_schedule=schedule,execution_controls=controls,observation=obs)
    result=prepared.execute(sinks=[Barrier(d/'ready')],file_recorder=glassvm.FileRecorder(str(d/'published')))
    (d/'result.json').write_text(json.dumps(result,indent=2))

def main():
    p=argparse.ArgumentParser();p.add_argument('--child',choices=['chip8','pico8','tic80']);p.add_argument('--dir',type=Path);p.add_argument('--output-dir',type=Path,default=OUT);a=p.parse_args()
    if a.child: child(a.child,a.dir);return
    if a.output_dir.exists() and any(a.output_dir.iterdir()):raise SystemExit('output directory not empty')
    a.output_dir.mkdir(parents=True,exist_ok=True);rows=[]
    for machine in ('chip8','pico8','tic80'):
        for rep in range(1,4):
            with tempfile.TemporaryDirectory(prefix='glassvm-e1-f5-') as tmp:
                d=Path(tmp);proc=subprocess.Popen([sys.executable,'-I',str(Path(__file__).resolve()),'--child',machine,'--dir',str(d)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
                deadline=time.monotonic()+30
                while not (d/'ready').exists() and proc.poll() is None and time.monotonic()<deadline:time.sleep(.02)
                ready=(d/'ready').exists();proc.kill();stdout,stderr=proc.communicate(timeout=5)
                files=[{'path':str(x.relative_to(d)),'bytes':x.stat().st_size} for x in d.rglob('*') if x.is_file()]
                row={'machine':machine,'case':'F5','repetition':rep,'ready':ready,'child_exit_code':proc.returncode,'published_exists':(d/'published').exists(),'returned_result':(d/'result.json').exists(),'staging_exists':(d/'.staging').exists(),'files':files,'stdout':stdout,'stderr':stderr,'request':json.loads((d/'request.json').read_text()) if (d/'request.json').exists() else None}
            (a.output_dir/f'{machine}-r{rep}.json').write_text(json.dumps(row,indent=2,sort_keys=True)+'\n');rows.append(row)
            ok=ready and proc.returncode<0 and row['staging_exists'] and not row['published_exists'] and not row['returned_result']
            print(machine,rep,'pass' if ok else 'unexpected',flush=True)
            if not ok:
                (a.output_dir/'partial.json').write_text(json.dumps(rows,indent=2)+'\n');raise SystemExit('unexpected F5 outcome')
    rev=lambda root:subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip(); wheels=Path('/tmp/glassvm-f1-local-wheels')
    m={'case':'F5','source':{'glassvm':rev(ROOT),'machines':rev(MACHINES)},'packages':{x:importlib.metadata.version(x) for x in ('glassvm','glassvm-chip8','glassvm-pico8','glassvm-tic80')},'wheel_sha256':{x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in wheels.glob('*.whl')},'host':platform.platform(),'python':sys.version,'runner_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'trials':len(rows),'rows':rows}
    (a.output_dir/'manifest.json').write_text(json.dumps(m,indent=2,sort_keys=True)+'\n')
if __name__=='__main__':main()
