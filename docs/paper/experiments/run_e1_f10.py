#!/usr/bin/env python3
"""F10: distinguish a completed zero-item frame channel from truncation."""
import argparse,hashlib,importlib.metadata,importlib.util,json,platform,subprocess,sys,tempfile,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];MACHINES=ROOT.parent/'glassvm-machines';HELPER=MACHINES/'conformance/paper/ce1/run_ce1.py';OUT=ROOT/'docs/paper/experiments/measurements/e1-f10-local-repair'
def worker(machine,rep):
 spec=importlib.util.spec_from_file_location('ce1_helpers',HELPER);h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h);base=MACHINES/machine/'fixtures/paper'
 if machine!='chip8':base/='ce1'
 c=tomllib.loads((base/'cohort.toml').read_text());w=c['workload'][0];program=(MACHINES/machine/'fixtures'/w['artifact_path']).read_bytes();digest=hashlib.sha256(program).hexdigest()
 if digest!=w['sha256']:raise ValueError('fixture digest mismatch')
 obs=h.observation_request({'normalized_events':'none','frames':'hashes'},c);config=h.machine_configuration(machine,c['configuration']);schedule={'schema':h.INPUT_SCHEDULE_SCHEMA,'entries':[]};controls={'frame_limit':0,'step_limit':None,'bundle_limits':[]}
 import glassvm
 prepared=glassvm.Runtime.discover().prepare(machine,program,configuration=config,input_schedule=schedule,execution_controls=controls,observation=obs)
 with tempfile.TemporaryDirectory(prefix='glassvm-e1-f10-') as d:
  result=prepared.execute(file_recorder=glassvm.FileRecorder(d+'/published'));files=[str(x.relative_to(d)) for x in Path(d).rglob('*') if x.is_file()]
 return {'case':'F10-zero','machine':machine,'repetition':rep,'artifact':{'path':w['artifact_path'],'sha256':digest},'request':{'configuration':config,'schedule':schedule,'controls':controls,'observation':obs},'result':result,'file_paths_before_cleanup':files}
def main():
 p=argparse.ArgumentParser();p.add_argument('--worker',choices=['chip8','pico8','tic80']);p.add_argument('--repetition',type=int);p.add_argument('--output-dir',type=Path,default=OUT);a=p.parse_args()
 if a.worker:print(json.dumps(worker(a.worker,a.repetition)));return
 if a.output_dir.exists() and any(a.output_dir.iterdir()):raise SystemExit('output directory not empty')
 a.output_dir.mkdir(parents=True,exist_ok=True);rows=[]
 for machine in ('chip8','pico8','tic80'):
  for rep in range(1,4):
   proc=subprocess.run([sys.executable,'-I',str(Path(__file__).resolve()),'--worker',machine,'--repetition',str(rep)],text=True,capture_output=True)
   row=json.loads(proc.stdout) if proc.returncode==0 else {'machine':machine,'repetition':rep,'worker_failure':{'returncode':proc.returncode,'stdout':proc.stdout,'stderr':proc.stderr}}
   (a.output_dir/f'{machine}-r{rep}.json').write_text(json.dumps(row,indent=2,sort_keys=True)+'\n');rows.append(row);r=row.get('result',{});ev=r.get('evidence_fulfillment') or {};frame=next((x for x in ev.get('channels',[]) if x['channel']=='frames'),{})
   ok=((r.get('machine_execution') or {}).get('status')=='completed' and ev.get('status')=='complete' and frame.get('requested') is True and frame.get('status')=='complete' and frame.get('item_count')==0 and (r.get('recorder_finalization') or {}).get('status')=='finalized' and (r.get('publication') or {}).get('status')=='published')
   print(machine,rep,'pass' if ok else 'unexpected',flush=True)
   if not ok:(a.output_dir/'partial.json').write_text(json.dumps(rows,indent=2)+'\n');raise SystemExit('unexpected F10 outcome')
 rev=lambda root:subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip();wheels=Path('/tmp/glassvm-f1-local-wheels')
 m={'case':'F10-zero','source':{'glassvm':rev(ROOT),'machines':rev(MACHINES)},'packages':{x:importlib.metadata.version(x) for x in ('glassvm','glassvm-chip8','glassvm-pico8','glassvm-tic80')},'wheel_sha256':{x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in wheels.glob('*.whl')},'host':platform.platform(),'python':sys.version,'runner_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'trials':len(rows),'comparison':'F6 allow_incomplete normalized-event trials','rows':rows}
 (a.output_dir/'manifest.json').write_text(json.dumps(m,indent=2,sort_keys=True)+'\n')
if __name__=='__main__':main()
