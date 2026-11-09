#!/usr/bin/env python3
"""F9: run deterministic malformed programs and preserve machine outcomes."""
import argparse,hashlib,importlib.metadata,importlib.util,json,platform,subprocess,sys,tempfile,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];MACHINES=ROOT.parent/'glassvm-machines';HELPER=MACHINES/'conformance/paper/ce1/run_ce1.py';OUT=ROOT/'docs/paper/experiments/measurements/e1-f9-local-repair'
class Counts:
 def __init__(self):self.counts={}
 def emit_batch(self,values):
  for value in values:self.counts[value['kind']]=self.counts.get(value['kind'],0)+1
 def record_capability_outputs(self,values):pass
 def record_capability_receipts(self,values):pass
def artifact(machine):
 lua=b'-- script: lua\nfunction TIC() error("E1 F9 runtime fault") end\n'
 if machine=='chip8':return bytes.fromhex('ffff')
 if machine=='pico8':return b'pico-8 cartridge // http://www.pico-8.com\nversion 41\n__lua__\nfunction _draw() error("E1 F9 runtime fault") end\n'
 return bytes([5])+len(lua).to_bytes(2,'little')+bytes([0])+lua
def worker(machine,rep):
 spec=importlib.util.spec_from_file_location('ce1_helpers',HELPER);h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h);base=MACHINES/machine/'fixtures/paper'
 if machine!='chip8':base/='ce1'
 c=tomllib.loads((base/'cohort.toml').read_text());program=artifact(machine);obs=h.observation_request({'normalized_events':'all','frames':'none'},c);config=h.machine_configuration(machine,c['configuration']);schedule={'schema':h.INPUT_SCHEDULE_SCHEMA,'entries':[]};controls={'frame_limit':2,'step_limit':None,'bundle_limits':[]}
 import glassvm
 prepared=glassvm.Runtime.discover().prepare(machine,program,configuration=config,input_schedule=schedule,execution_controls=controls,observation=obs);sink=Counts()
 with tempfile.TemporaryDirectory(prefix='glassvm-e1-f9-') as d:
  result=prepared.execute(sinks=[sink],file_recorder=glassvm.FileRecorder(d+'/published'));files=[str(x.relative_to(d)) for x in Path(d).rglob('*') if x.is_file()]
 return {'case':'F9','machine':machine,'repetition':rep,'artifact':{'hex':program.hex(),'sha256':hashlib.sha256(program).hexdigest()},'request':{'configuration':config,'schedule':schedule,'controls':controls,'observation':obs},'counts':sink.counts,'result':result,'file_paths_before_cleanup':files}
def main():
 p=argparse.ArgumentParser();p.add_argument('--worker',choices=['chip8','pico8','tic80']);p.add_argument('--repetition',type=int);p.add_argument('--output-dir',type=Path,default=OUT);a=p.parse_args()
 if a.worker:print(json.dumps(worker(a.worker,a.repetition)));return
 if a.output_dir.exists() and any(a.output_dir.iterdir()):raise SystemExit('output directory not empty')
 a.output_dir.mkdir(parents=True,exist_ok=True);rows=[]
 for machine in ('chip8','pico8','tic80'):
  for rep in range(1,4):
   proc=subprocess.run([sys.executable,'-I',str(Path(__file__).resolve()),'--worker',machine,'--repetition',str(rep)],text=True,capture_output=True)
   row=json.loads(proc.stdout) if proc.returncode==0 else {'machine':machine,'repetition':rep,'worker_failure':{'returncode':proc.returncode,'stdout':proc.stdout,'stderr':proc.stderr}}
   (a.output_dir/f'{machine}-r{rep}.json').write_text(json.dumps(row,indent=2,sort_keys=True)+'\n');rows.append(row);r=row.get('result',{});ex=r.get('machine_execution') or {};ev=r.get('evidence_fulfillment') or {};deliver=r.get('python_sink_delivery') or {};rec=r.get('recorder_finalization') or {};pub=r.get('publication') or {}
   if machine=='chip8':ok=ex.get('status')=='completed' and ex.get('result',{}).get('common',{}).get('termination')=='InvalidOpcode' and ev.get('status')=='complete' and rec.get('status')=='finalized' and pub.get('status')=='published'
   else:ok=ex.get('status')=='failed' and 'E1 F9 runtime fault' in (ex.get('result',{}).get('message') or '') and rec.get('status')=='not_finalized' and pub.get('status')=='not_attempted' and ev.get('status') in ('complete','incomplete')
   ok=ok and deliver.get('status')=='complete' and row.get('counts',{}).get('run_started',0)==1
   print(machine,rep,'pass' if ok else f'unexpected {ex}',flush=True)
   if not ok:(a.output_dir/'partial.json').write_text(json.dumps(rows,indent=2)+'\n');raise SystemExit('unexpected F9 outcome')
 rev=lambda root:subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip();wheels=Path('/tmp/glassvm-f1-local-wheels')
 m={'case':'F9','source':{'glassvm':rev(ROOT),'machines':rev(MACHINES)},'packages':{x:importlib.metadata.version(x) for x in ('glassvm','glassvm-chip8','glassvm-pico8','glassvm-tic80')},'wheel_sha256':{x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in wheels.glob('*.whl')},'host':platform.platform(),'python':sys.version,'runner_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'trials':len(rows),'rows':rows}
 (a.output_dir/'manifest.json').write_text(json.dumps(m,indent=2,sort_keys=True)+'\n')
if __name__=='__main__':main()
