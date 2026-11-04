import subprocess,json,time,os,hashlib,signal,shutil
from pathlib import Path
ROOT=Path('/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-17-terminal-discovery');BASE=Path('/tmp/gibbon17-40ca7f-unlocked');OUT=ROOT/'docs/verification/terminal-discovery-unlocked-20261105';CLI=ROOT/'scripts/agent-gibbon-dev'
INFO=json.loads(subprocess.check_output(['node',str(ROOT/'scripts/dev.mjs'),'info'],text=True));REPORT=OUT/'run.json'
def read(): return json.loads(REPORT.read_text()) if REPORT.exists() else {'dev':INFO,'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'sha256':hashlib.sha256(Path(INFO['binary']).read_bytes()).hexdigest(),'actions':[]}
def save(r):REPORT.write_text(json.dumps(r,ensure_ascii=False,indent=2))
def cli(*args,env=None):
 t=time.monotonic_ns();p=subprocess.run([str(CLI),*args],env=env,cwd=ROOT,text=True,capture_output=True,timeout=20);end=time.monotonic_ns();r=json.loads(p.stdout);assert p.returncode==0 and r['ok'] and r['instance']==INFO['id'],(p,r)
 d=read();d['actions'].append({'args':args,'start_ns':t,'end_ns':end,'seconds':(end-t)/1e9,'response':r});save(d);return r
def pids():return [int(l.split(None,1)[0]) for l in subprocess.check_output(['/bin/ps','-axo','pid=,command='],text=True).splitlines() if l.strip().split(None,1)[-1]==INFO['binary']+' --serve']
def start(label,limited):
 assert not pids(); env=os.environ.copy();env['SHELL']=str(BASE/'discovery-shell');env['PATH']=('' if limited else str(BASE/'bin')+':')+str(BASE/'tools')+':/usr/bin:/bin'
 cli('open','--id','issue17-unlocked-progress','--title','#17 최종 앱 검증','--markdown','# 터미널 탐색 개선 #17\n- [x] HEAD·CI·제품 코드·빌드 출처 확인\n- [ ] 최종 앱 표시·네이티브 입력·resize·숨김 복원\n- [ ] 느린 탐색 중 기존 입력·resize·조회\n- [ ] 캐시·제한 PATH 검증\n- [ ] 문서·커밋·감독 보고',env=env)
 ps=pids();assert len(ps)==1;d=read();d[label]={'pid':ps[0],'env_path':env['PATH'],'shell':env['SHELL'],'command':subprocess.check_output(['/bin/ps','-p',str(ps[0]),'-o','pid,ppid,comm'],text=True),'marker':cli('inspect','--id','issue17-unlocked-progress')};save(d);print(label,ps[0])
def agent(label):
 t=time.monotonic();a=cli('agent');deadline=t+12
 while not (v:=cli('inspect','--id',a['id']))['running']:assert time.monotonic()<deadline;time.sleep(.03)
 d=read();d[label]={'id':a['id'],'until_running_seconds':time.monotonic()-t,'inspect':v};save(d);print(label,a['id'],d[label]['until_running_seconds']);return a['id']
def stop():
 for p in pids():os.kill(p,signal.SIGTERM)
 until=time.monotonic()+10
 while pids():assert time.monotonic()<until;time.sleep(.1)
if __name__=='__main__':
 import sys
 if sys.argv[1]=='start':start(sys.argv[2],sys.argv[3]=='limited')
 elif sys.argv[1]=='agent':agent(sys.argv[2])
 elif sys.argv[1]=='stop':stop()
