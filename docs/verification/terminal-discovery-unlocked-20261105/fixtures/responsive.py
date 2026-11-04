import subprocess,json,time,threading
from pathlib import Path
BASE=Path('/tmp/gibbon17-40ca7f-unlocked');ROOT=Path('/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-17-terminal-discovery');OUT=ROOT/'docs/verification/terminal-discovery-unlocked-20261105';CLI=ROOT/'scripts/agent-gibbon-dev';PID=62412;WIN=33969;OLD='agent-1793833831951440000'
def call(args):
 t=time.time();m=time.monotonic();p=subprocess.run(args,capture_output=True,text=True,timeout=15);r={'args':list(map(str,args)),'start_at':t,'end_at':time.time(),'seconds':time.monotonic()-m,'exit':p.returncode,'response':json.loads(p.stdout)};
 if p.returncode!=0 or not r['response']['ok']:(OUT/'responsive-error.json').write_text(json.dumps(r,ensure_ascii=False,indent=2));raise RuntimeError(r)
 return r
(BASE/'shell-config.json').write_text(json.dumps({'delay':2.2,'path':str(BASE/'targetbin')+':/usr/bin:/bin'}));(BASE/'fallback/codex').unlink(missing_ok=True); (BASE/'shell-events.jsonl').write_text('')
r={'trigger':call([str(CLI),'agent'])};new=r['trigger']['response']['id'];r['hide_new']=call([str(CLI),'close','--id',new]);deadline=time.time()+3
while not (BASE/'shell-events.jsonl').read_text():assert time.time()<deadline;time.sleep(.01)
r['observed_shell_begin']=json.loads((BASE/'shell-events.jsonl').read_text().splitlines()[-1]);r['inspects']=[];stop=threading.Event()
def poll():
 while not stop.is_set():
  r['inspects'].append(call([str(CLI),'inspect','--id',OLD]));time.sleep(.04)
th=threading.Thread(target=poll,daemon=True);th.start();r['gui']=[]
for label,op,args in [('slow-key1','press-key',['--key','1']),('slow-return','press-key',['--key','Return']),('slow-resize','drag',['--from-x','717','--from-y','457','--to-x','857','--to-y','547'])]:
 a=call(['orca','computer',op,'--app',f'pid:{PID}','--window-id',str(WIN),*args,'--no-screenshot','--json']);r['gui'].append(a);(OUT/(label+'.json')).write_text(json.dumps(a,ensure_ascii=False,indent=2))
while not (v:=call([str(CLI),'inspect','--id',new]))['response']['running']:time.sleep(.03)
r['new_running']=v;stop.set();th.join();r['old_final']=call([str(CLI),'inspect','--id',OLD]);r['shell_events']=[json.loads(l) for l in (BASE/'shell-events.jsonl').read_text().splitlines()];r['fake_events']=[json.loads(l) for l in (BASE/'fake-events.jsonl').read_text().splitlines()];(OUT/'responsive.json').write_text(json.dumps(r,ensure_ascii=False,indent=2));print('new',new,'inspect count',len(r['inspects']),'max',max(v['seconds'] for v in r['inspects']),'gui seconds',[v['seconds'] for v in r['gui']]);print('shell',r['shell_events']);print('old during',[v for v in r['fake_events'] if v['pid']==62893 and v['at']>=r['observed_shell_begin']['at']])
