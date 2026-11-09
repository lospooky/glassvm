#!/usr/bin/env python3
"""Evaluate F7 from the existing F6 allow-incomplete trials."""
import json
from pathlib import Path
root=Path(__file__).resolve().parent/'measurements';source=root/'e1-f6-local-repair';target=root/'e1-f7-local-repair.json'
rows=[]
for path in sorted(source.glob('*-allow_incomplete-r*.json')):
    trial=json.loads(path.read_text());result=trial['result'];rec=result['recorder_finalization']['result'];ev=result['evidence_fulfillment'];published=result.get('published_run') or {};files=set(trial['file_paths_before_cleanup'])
    checks={'machine_completed':result['machine_execution']['status']=='completed','evidence_incomplete':ev['status']=='incomplete','recording_complete':rec['status']=='complete' and rec['finalized'],'recorder_reports_incomplete_evidence':rec['evidence_status']=='incomplete','evidence_receipt_published':rec['evidence_receipt_published'],'published':result['publication']['status']=='published' and published.get('published') is True,'receipt_files_present':{'published/evidence-receipt.json','published/recorder-receipt.json'}.issubset(files)}
    rows.append({'source':str(path.relative_to(root)),'machine':trial['machine'],'repetition':trial['repetition'],'checks':checks})
    if not all(checks.values()):raise SystemExit(f'unexpected F7 outcome: {path} {checks}')
if len(rows)!=9:raise SystemExit(f'expected 9 trials, found {len(rows)}')
target.write_text(json.dumps({'case':'F7','source':'E1 F6 allow_incomplete trials; no new executions','count':len(rows),'rows':rows},indent=2,sort_keys=True)+'\n')
print('F7 passed: 9 finalized, published recordings with incomplete evidence')
