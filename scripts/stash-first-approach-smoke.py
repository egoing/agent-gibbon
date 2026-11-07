#!/usr/bin/env python3
"""macOS real-pointer regression: run with before or after on an idle dev instance.
Creates only a unique test memo; caller owns the dev process lifecycle.
The existing pointer helper emits mouseMoved without clicks and restores position.
"""
import json,subprocess,time,sys,tempfile,uuid
from pathlib import Path
R=Path(__file__).resolve().parents[1]; phase=sys.argv[1]; assert phase in ('before','after'); O=R/'docs/verification'/('issue36-'+phase);O.mkdir(parents=True,exist_ok=True)
# The current regression includes first approach AND immediate later reentry.
# Keep the legacy before mode only for historical pre-#36 reproductions.
if phase == 'after':
 subprocess.run([sys.executable, str(R/'scripts/stash-reentry-smoke.py'), 'after'], check=True)
 sys.exit(0)
tmp=tempfile.TemporaryDirectory(prefix='gibbon-first-approach-');H=Path(tmp.name); ID='issue36-first-approach-'+uuid.uuid4().hex[:8]; info=json.loads(subprocess.check_output(['node',str(R/'scripts/dev.mjs'),'info'])); events=[]; t=time.monotonic()
def run(*a):return subprocess.check_output(list(map(str,a)),text=True)
def helper(n,*a):return run(H/n,*a)
def cli(action,*a):
 r=json.loads(run(R/'scripts/agent-gibbon-dev',action,'--id',ID,*a));assert r['ok'] and r['instance']==info['id'];return r
def state(label):
 r=cli('inspect');events.append({'at':round(time.monotonic()-t,3),'label':label,'stash':r['stash'],'focused':r['focused']});return r['stash']
def wait(pred,label,timeout=10):
 end=time.monotonic()+timeout
 while time.monotonic()<end:
  s=state(label)
  if pred(s):return s
  time.sleep(.025)
 raise AssertionError((label,s))
def move(x,y,label):
 events.append({'at':round(time.monotonic()-t,3),'pointer':[x,y],'label':label});helper('pointer','move',x,y)
def center(s,label):
 r=s['indicator'];move(r['x']+r['width']-24,r['y']+24,label)
def shot(name):
 native=json.loads(helper('native',pid));w=next(w for w in native['windows'] if w.get('kCGWindowName')==title);assert w['kCGWindowOwnerPID']==pid
 (O/(name+'-source.json')).write_text(json.dumps(w,indent=2));subprocess.run(['screencapture','-x','-o','-l',str(w['kCGWindowNumber']),str(O/(name+'.png'))],check=True)
for src,name in [('entrance-pointer.swift','pointer'),('stash-key.swift','key'),('auto-layout-native.swift','native'),('frontmost.swift','front')]:
 subprocess.run(['swiftc',str(R/'scripts'/src),'-o',str(H/name)],check=True)
original=json.loads(helper('pointer'));focus=helper('front');report={'root':str(R),'info':info,'id':ID,'phase':phase,'events':events,'focus_before':focus}
try:
 cli('open','--markdown',ID);r=cli('inspect'); assert ID in r['note']['markdown'];pid=next(int(l.split(maxsplit=1)[0]) for l in run('ps','-axo','pid=,command=').splitlines() if len(l.strip().split(maxsplit=1))==2 and l.strip().split(maxsplit=1)[1]==info['binary']+' --serve');report['pid']=pid; report['executable']=info['binary'];title='Agent Gibbon ['+r['dev_label']+'] — stash-indicator'
 assert not r['stash']['active'];assert r['stash']['shortcut']=='Ctrl+Alt+Cmd','Shortcut owned elsewhere; do not send it';move(600,500,'away');time.sleep(1.3);helper('key')
 initial=wait(lambda s:s['active'] and not s['animating'] and s['tip_visible'],'initial hidden')
 time.sleep(.12);initial=state('initial native placement settled');center(initial,'first approach during initial bubble');time.sleep(.16);first=state('first response');shot('first-response')
 if phase=='before':
  assert not first['indicator_moving'] and first['indicator']==initial['indicator'];report['reproduced']='first approach blocked during initial bubble'
  arrived=wait(lambda s:not s['indicator_moving'] and s['indicator']['y']!=initial['indicator']['y'] and s['tip_visible'],'held pointer delayed flight')
 else:
  assert first['indicator_moving'] and not first['tip_visible'];arrived=wait(lambda s:not s['indicator_moving'] and s['tip_visible'],'arrived')
 shot('arrived');assert arrived['indicator']['y']!=initial['indicator']['y']
 center(arrived,'arrival approach during bubble');time.sleep(.3);s=state('arrival protected');assert not s['indicator_moving'] and s['indicator']==arrived['indicator']
 move(600,500,'96px exit during bubble');center(arrived,'reentry during bubble');time.sleep(.3);assert state('bubble reentry protected')['indicator']==arrived['indicator']
 wait(lambda s:not s['tip_visible'],'bubble expired');time.sleep(.15);assert state('held at destination no chase')['indicator']==arrived['indicator']
 move(600,500,'96px exit after bubble');time.sleep(.12);center(arrived,'reentry after bubble');moving=wait(lambda s:s['indicator_moving'],'second flight');assert not moving['tip_visible']
 second=wait(lambda s:not s['indicator_moving'] and s['tip_visible'],'second arrival');assert second['indicator']['y']==initial['indicator']['y']
 assert helper('front')==focus;report['focus_after']=helper('front');helper('key');wait(lambda s:not s['active'],'restored');assert cli('inspect')['visible'];report['result']='passed'
finally:
 try:
  if cli('inspect')['stash']['active']:helper('key');time.sleep(.6)
 finally:
  cli('close');move(original['x'],original['y'],'restore original pointer');(O/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2));print(O)
