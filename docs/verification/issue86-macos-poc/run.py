import subprocess,json,time,pathlib,hashlib,os
D=pathlib.Path(__file__).resolve().parent; BIN=D/'probe'; report={'events':[]};procs=[]
def call(*a):
 p=subprocess.run(list(map(str,a)),capture_output=True,text=True,timeout=20)
 report['events'].append({'command':list(map(str,a)),'exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr})
 if p.returncode: raise RuntimeError(p.stderr or p.stdout)
 return p.stdout
def probe(*a):return json.loads(call(BIN,*a))
def state(p):return probe('state',p.pid)
def wait_state(p):
 for _ in range(40):
  f=D/(p.tag+'.json')
  if f.exists():
   v=json.loads(f.read_text())
   if v['pid']==p.pid:
    time.sleep(.5);return v
  time.sleep(.1)
 raise RuntimeError('fixture never ready')
def snapshot(name):
 s=probe('order',A.pid,B.pid);report[name]=s
 (D/(name+'.json')).write_text(json.dumps(s,indent=2))
 call('screencapture','-x','-R220,180,560,360',D/(name+'.png'))
 return s
def activate(p):
 report['events'].append({'activate':probe('activate',p.pid)})
 time.sleep(.4)
 assert state(p)['frontPID']==p.pid, 'activation not observed'
try:
 report['environment']=env=probe('environment');report['sourceSHA']=call('git','rev-parse','HEAD').strip()
 report['os']=call('sw_vers');report['swift']=call('swift','--version')
 report['sourceHash']=hashlib.sha256((D/'probe.swift').read_bytes()).hexdigest()
 for tag in ['A','B']:
  p=subprocess.Popen([str(BIN),'fixture',tag,str(D/(tag+'.json'))],stdout=open(D/(tag+'.stdout'),'w'),stderr=open(D/(tag+'.stderr'),'w'));p.tag=tag;procs.append(p);wait_state(p)
 A,B=procs
 report['identities']=[{'pid':p.pid,'exe':call('ps','-p',p.pid,'-o','comm=').strip(),'initial':state(p)} for p in procs]
 original=report['identities'][0]['initial']['windows'][0]['kCGWindowBounds'];report['original']=original
 report['moveAPI']=probe('move',A.pid,220,180,560,360);time.sleep(.3)
 report['moved']=state(A)
 assert report['moved']['windows'][0]['kCGWindowBounds']=={'X':220,'Y':180,'Width':560,'Height':360}
 report['movement']='PASS'
 probe('move',B.pid,320,230,560,360);time.sleep(.3)
 activate(A)
 probe('input',A.pid,340,420);time.sleep(.3)
 probe('input',A.pid,370,350,'MAC86_REAL_INPUT');time.sleep(.5)
 report['realInput']=wait_state(A)
 assert report['realInput']['clicks']==1 and report['realInput']['text']=='MAC86_REAL_INPUT', report['realInput']
 report['input']='PASS';snapshot('01-A-input')
 report['levelProbe']=probe('level',A.pid)
 activate(B)
 report['raiseAPI']=probe('raise',A.pid);time.sleep(.3);raised=snapshot('02-A-raised')
 report['backgroundRaiseObserved']=raised['frontToBack'][0]['kCGWindowOwnerPID']==A.pid and raised['frontPID']==B.pid
 probe('input',B.pid,830,400);time.sleep(.3)
 probe('input',B.pid,440,470);time.sleep(.3)
 report['B-real-click']=wait_state(B)
 assert report['B-real-click']['clicks']==1
 observed=snapshot('03-B-active-after-A-raise')
 report['topmost']='PASS' if observed['frontToBack'][0]['kCGWindowOwnerPID']==A.pid else 'FAIL'
 report['after-one-second']=probe('order',A.pid,B.pid) if not time.sleep(1) else None
 report['restoreAPI']=probe('move',A.pid,original['X'],original['Y'],original['Width'],original['Height']);time.sleep(.3)
 report['restored']=state(A);assert report['restored']['windows'][0]['kCGWindowBounds']==original
 report['restoreGeometry']='PASS';report['unpin']='NOT_APPLICABLE: no persistent pin obtained; no level changed'
 report['status']='COMPLETED_WITH_TOPMOST_FAILURE'
except Exception as e:
 report['error']=repr(e);report['status']='INCOMPLETE';print('RUN ERROR',repr(e),flush=True)
finally:
 for p in procs:
  if p.poll() is None:
   p.terminate()
   try:p.wait(timeout=5)
   except subprocess.TimeoutExpired:p.kill();p.wait()
 report['cleanup']=[{'pid':p.pid,'exit':p.poll()} for p in procs]
 if 'env' in locals():
  report['focusRestore']=probe('activate',env['frontPID']);call(BIN,'restorePointer',*env['pointer']);time.sleep(.3);report['finalEnvironment']=probe('environment')
 (D/'result.json').write_text(json.dumps(report,ensure_ascii=False,indent=2))
 print(json.dumps({k:v for k,v in report.items() if k in ['status','error','movement','input','topmost','restoreGeometry','unpin','realInput','levelProbe','cleanup','finalEnvironment']},ensure_ascii=False,indent=2))

raise SystemExit(0 if report.get('topmost')=='PASS' and report.get('status')!='INCOMPLETE' else 2)
