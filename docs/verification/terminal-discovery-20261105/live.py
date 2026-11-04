import subprocess,json,time,os,hashlib,signal
from pathlib import Path
ROOT=Path.cwd();OUT=Path('/tmp/gibbon-issue17');CLI=ROOT/'scripts/agent-gibbon-dev';INFO=json.loads(subprocess.check_output(['node','scripts/dev.mjs','info']))
def cli(*args,env=None):
 r=json.loads(subprocess.check_output([str(CLI),*args],env=env,text=True,timeout=20));assert r['ok'] and r['instance']==INFO['id'];return r
def pids():
 return [int(l.split(None,1)[0]) for l in subprocess.check_output(['/bin/ps','-axo','pid=,command='],text=True).splitlines() if l.strip().split(None,1)[-1]==INFO['binary']+' --serve']
def stop():
 for pid in pids():os.kill(pid,signal.SIGTERM)
 end=time.monotonic()+10
 while pids():assert time.monotonic()<end;time.sleep(.1)
def case(label,limited=False):
 stop();(OUT/'shell.log').write_text('');(OUT/'cli.log').write_text('')
 env=os.environ.copy();env['SHELL']=str(OUT/'shell');env['PATH']=('' if limited else str(OUT/'bin')+':')+'/usr/bin:/bin:/opt/homebrew/bin'
 cli('open','--id','issue17-progress',env=env);pid=pids();assert len(pid)==1;pid=pid[0]
 marker=cli('inspect','--id','issue17-progress');assert marker['note']['markdown'].startswith('# 터미널 탐색 개선 #17')
 report={'info':INFO,'pid':pid,'command':subprocess.check_output(['/bin/ps','-p',str(pid),'-o','command='],text=True).strip(),'sha256':hashlib.sha256(Path(INFO['binary']).read_bytes()).hexdigest(),'marker':marker,'label':label,'limited_path':limited,'started':time.time(),'sessions':[]}
 def agent():
  t=time.monotonic();v=cli('agent');created=time.monotonic()-t
  end=time.monotonic()+12
  while not (r:=cli('inspect','--id',v['id']))['running']:assert time.monotonic()<end;time.sleep(.05)
  report['sessions'].append({'id':v['id'],'create_seconds':created,'until_running_seconds':time.monotonic()-t,'inspect':r});return v['id']
 a=agent();time.sleep(.3);b=agent();time.sleep(2)
 report['native']=json.loads(subprocess.check_output([str(OUT/'native'),str(pid)]));
 for s in report['sessions']:
  w=next(w for w in report['native']['windows'] if w.get('kCGWindowName','').endswith(' — '+s['id']));assert w['kCGWindowOwnerPID']==pid;s['native']=w
  subprocess.run(['screencapture','-x','-o','-l',str(w['kCGWindowNumber']),str(OUT/(label+'-'+s['id']+'.png'))],check=True)
  s['final']=cli('inspect','--id',s['id']);cli('close','--id',s['id']);closed=cli('inspect','--id',s['id']);assert not closed['visible'] and closed['pid']==s['final']['pid'];cli('open','--id',s['id']);restored=cli('inspect','--id',s['id']);assert restored['visible'] and restored['pid']==s['final']['pid'];s['hidden']=closed;s['restored']=restored
 report['shell_log']=(OUT/'shell.log').read_text();report['cli_log']=(OUT/'cli.log').read_text();(OUT/(label+'.json')).write_text(json.dumps(report,ensure_ascii=False,indent=2));print(json.dumps({'label':label,'pid':pid,'shell':report['shell_log'],'times':[r['until_running_seconds'] for r in report['sessions']],'observer_lines':len(report['cli_log'].splitlines())},ensure_ascii=False));return report
if __name__=='__main__':
 import sys
 case(sys.argv[1],len(sys.argv)>2)
