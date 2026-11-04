import subprocess,json,sys,shutil,time
from pathlib import Path
out=Path('/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-17-terminal-discovery/docs/verification/terminal-discovery-unlocked-20261105')
label,op,*args=sys.argv[1:];t=time.monotonic_ns();p=subprocess.run(['orca','computer',op,*args,'--json'],text=True,capture_output=True);end=time.monotonic_ns();r=json.loads(p.stdout);r['fixture_timing']={'start_ns':t,'end_ns':end,'seconds':(end-t)/1e9};(out/(label+'.json')).write_text(json.dumps(r,ensure_ascii=False,indent=2));print('ok',r.get('ok'),'seconds',(end-t)/1e9)
s=r.get('result',{}).get('screenshot') or {};path=s.get('path')
if path:shutil.copy(path,out/(label+'.png'));print('image',str(out/(label+'.png')))
print(json.dumps({k:v for k,v in r.get('result',{}).items() if k!='screenshot'},ensure_ascii=False)[:12000])
