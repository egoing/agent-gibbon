#!/usr/bin/env python3
"""Real macOS shortcut, native windows, cursor, AX draft and instance regression.
Only creates unique notes through this worktree's dev CLI. Never stops any app.
"""
import json, subprocess, tempfile, time, uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
CLI=str(ROOT/'scripts/agent-gibbon-dev')
INFO=json.loads(subprocess.check_output(['node',str(ROOT/'scripts/dev.mjs'),'info'],text=True))
assert INFO['root']==str(ROOT)
PREFIX='stash-smoke-'+uuid.uuid4().hex[:8]
OUT=ROOT/'docs/verification'/PREFIX
OUT.mkdir(parents=True)
TMP=tempfile.TemporaryDirectory(prefix='gibbon-stash-test-')
TEMP=Path(TMP.name)
for src,name in [('design-system-ax.swift','ax'),('entrance-pointer.swift','pointer'),('auto-layout-native.swift','native'),('frontmost.swift','front'),('stash-key.swift','key')]:
    subprocess.run(['swiftc',str(ROOT/'scripts'/src),'-o',str(TEMP/name)],check=True)
def helper(name,*args):return subprocess.check_output([str(TEMP/name),*map(str,args)],text=True)
def cli(action,id,*args):
    d=json.loads(subprocess.check_output([CLI,action,'--id',id,*args],text=True))
    assert d['ok'],d
    if action=='inspect':assert d['instance']==INFO['id'] and d['id']==id,d
    return d
def inspect(id):return cli('inspect',id)
def wait(id,predicate):
    deadline=time.monotonic()+8
    while time.monotonic()<deadline:
        d=inspect(id)
        if predicate(d):return d
        time.sleep(.04)
    raise AssertionError(d)
def key():helper('key')
def front():return int(helper('front').strip())
def move(x,y):helper('pointer','move',x,y)
def native():return json.loads(helper('native',PID))
def ax(id,action,*args):return helper('ax',PID,TITLE+id,action,*args)
def save(name,data): (OUT/(name+'.json')).write_text(json.dumps(data,ensure_ascii=False,indent=2))
def shot(name,match):
    rows=native()['windows'];w=next(w for w in rows if match in w.get('kCGWindowName',''))
    assert w['kCGWindowOwnerPID']==PID
    save(name+'-source',w)
    subprocess.run(['screencapture','-x','-o','-l',str(w['kCGWindowNumber']),str(OUT/(name+'.png'))],check=True)
ids=[PREFIX+'-'+x for x in ['edit','fold','hidden','new']]
a,b,c,d=ids
pointer=json.loads(helper('pointer'))
report={'instance':INFO,'ids':ids,'checks':[]}
def passed(msg):report['checks'].append(msg);print(msg,flush=True)
try:
    cli('open',a,'--markdown',f'# {PREFIX}\n편집 원문')
    processes=subprocess.check_output(['ps','-axo','pid=,command='],text=True).splitlines()
    expected=INFO['binary']+' --serve'
    matches=[int(line.strip().split(maxsplit=1)[0]) for line in processes if line.strip().split(maxsplit=1)[-1]==expected]
    assert len(matches)==1,matches
    PID=matches[0];report['pid']=PID;report['executable']=expected
    TITLE='Agent Gibbon ['+inspect(a)['dev_label']+'] — '
    assert inspect(a)['stash']['shortcut']=='Ctrl+Alt+Cmd','Key is owned elsewhere; do not send it'
    if inspect(a)['stash']['active']:key();wait(a,lambda s:not s['stash']['active'])
    cli('open',b,'--markdown',f'# {PREFIX} folded\n접힘 보존')
    cli('open',c,'--markdown','원래 숨긴 창')
    cli('close',c)
    time.sleep(1.1);move(pointer['x']+20,pointer['y']+20)
    wait(a,lambda s:not s['animating']);wait(b,lambda s:not s['animating'])
    ax(a,'click','메모 편집');time.sleep(.2)
    draft=PREFIX+' 미저장 초안';assert ax(a,'draft',draft).strip()==draft
    ax(b,'click','메모 접기');wait(b,lambda s:s['folded']);time.sleep(.2)
    displays=native()['screens']
    if len(displays)>1:
        f=displays[1]['frame'];ax(b,'position',f[0]+100,displays[0]['frame'][3]-f[1]-f[3]+100)
    before={id:inspect(id) for id in [a,b,c]};save('before',before)
    assert PREFIX in before[a]['note']['markdown']
    report['native_before']=native()
    assert any(w.get('kCGWindowName')==TITLE+a for w in report['native_before']['windows'])
    passed('root/id/binary + inspect instance + PID/native title proven')
    shot('editor-before',a)
    # Cursor screen selection: left monitor in this host, otherwise primary.
    screens=report['native_before']['screens'];main_height=screens[0]['frame'][3]
    targets=[(s['frame'][0]+s['frame'][2]/2,main_height-s['frame'][1]-s['frame'][3]/2) for s in screens]
    target=targets[1] if len(targets)>1 else targets[0]
    move(*target);focus=front();report['focus_before']=focus
    key();frames=[]
    for _ in range(6):
        frames.append(native());time.sleep(.035)
    save('animation',frames)
    hidden=wait(a,lambda s:s['stash']['active'] and not s['stash']['animating'] and not s['visible'])
    assert not inspect(b)['visible'] and not inspect(c)['visible']
    assert front()!=PID,(focus,front())
    samples=[w for frame in frames for w in frame['windows'] if w.get('kCGWindowName')==TITLE+a]
    original=next(w for w in report['native_before']['windows'] if w.get('kCGWindowName')==TITLE+a)
    assert any(w['kCGWindowBounds']['Width']<original['kCGWindowBounds']['Width'] and w['kCGWindowAlpha']<original['kCGWindowAlpha'] for w in samples),samples
    passed('real hotkey: native movement/shrink/alpha/hide; original hidden stays hidden; focus preserved')
    r=wait(a,lambda s:s['stash']['indicator'] and s['stash']['indicator']['x']<0 if target[0]<0 else s['stash']['indicator'] is not None)['stash']['indicator']
    report['indicator_initial']=r
    wait(a,lambda s:s['stash']['tip_visible'])
    shot('indicator-initial','stash-indicator')
    assert '다시 누르면 창 복원' in helper('ax',PID,TITLE+'stash-indicator','dump')
    move(*target);time.sleep(6.2)
    move(r['x']+r['width']-24,r['y']+24)
    wait(a,lambda s:s['stash']['indicator_moving'])
    motion=[]
    for _ in range(5):
        state=inspect(a)['stash'];motion.append(state)
        if state['indicator_moving']:assert not state['tip_visible']
        time.sleep(.035)
    save('indicator-flight',motion)
    settled=wait(a,lambda s:not s['stash']['indicator_moving'] and s['stash']['tip_visible'])
    escaped=settled['stash']['indicator'];report['indicator_escaped']=escaped
    assert any(min(r['y'],escaped['y']) < v['indicator']['y'] < max(r['y'],escaped['y']) for v in motion),motion
    move(escaped['x']+escaped['width']-24,escaped['y']+24)
    wait(a,lambda s:s['stash']['indicator_moving'] and not s['stash']['tip_visible'])
    move(*target)
    escaped=wait(a,lambda s:not s['stash']['indicator_moving'] and s['stash']['tip_visible'])['stash']['indicator']
    move(*target);shot('indicator-tip','stash-indicator')
    text=helper('ax',PID,TITLE+'stash-indicator','dump')
    assert 'Control Option Command' in text and '다시 누르면 창 복원' in text,text
    move(*target);time.sleep(6.1)
    assert '다시 누르면 창 복원' not in helper('ax',PID,TITLE+'stash-indicator','dump')
    move(escaped['x']+escaped['width']-24,escaped['y']+24)
    wait(a,lambda s:s['stash']['indicator_moving']);move(*target)
    wait(a,lambda s:not s['stash']['indicator_moving'] and s['stash']['tip_visible'])
    passed('cursor-monitor icon, immediate reentry, independent 6s actual-key tip, no focus steal')
    cli('update',a,'--markdown',f'# {PREFIX}\n외부 갱신')
    assert not inspect(a)['visible']
    cli('open',d,'--markdown',PREFIX+' 새 창');wait(d,lambda s:s['visible'])
    assert not inspect(a)['visible'] and not inspect(b)['visible']
    key();restore_frames=[]
    for _ in range(5):restore_frames.append(native());time.sleep(.03)
    save('restore-animation',restore_frames)
    wait(a,lambda s:not s['stash']['active'] and s['visible'])
    samples=[w for f in restore_frames for w in f['windows'] if w.get('kCGWindowName')==TITLE+a]
    assert any(w['kCGWindowBounds']['Width']<original['kCGWindowBounds']['Width'] and w['kCGWindowAlpha']<original['kCGWindowAlpha'] for w in samples),samples
    passed('restoration animates position, size and alpha from the current indicator edge')
    for id in [a,b]:
        after=inspect(id)
        assert all(after[k]==before[id][k] for k in ['position','size','folded']), (before[id],after)
    assert not inspect(c)['visible'] and inspect(d)['visible']
    assert draft in ax(a,'dump')
    assert front()!=PID,(focus,front())
    save('restored',{id:inspect(id) for id in ids})
    passed('exact geometry/fold restoration, draft retained through CLI update, new open bypasses reflow')
    helper('key','mixed',PID);time.sleep(.2)
    assert not inspect(a)['stash']['active'],'Mixed shortcut must not toggle'
    held=subprocess.Popen([str(TEMP/'key'),'hold'])
    time.sleep(.45);assert not inspect(a)['stash']['active'],'Holding modifiers must not toggle'
    held.wait();wait(a,lambda s:not s['visible']);key();wait(a,lambda s:not s['stash']['active'])
    passed('other-key chord cancelled; holding alone does not fire; complete release toggles once')
    # Rapid reversal before animation completes, repeated generations.
    for _ in range(3):
        helper('key','fast');time.sleep(.06);helper('key','fast');wait(a,lambda s:not s['stash']['active']);time.sleep(.5)
        assert inspect(a)['position']==before[a]['position'] and inspect(a)['size']==before[a]['size']
    passed('three rapid animation reversals: no stale hide/move frame')
    key();time.sleep(.06);cli('close',b);wait(a,lambda s:not s['visible'])
    cli('open',a);assert inspect(a)['visible']
    key();wait(a,lambda s:not s['stash']['active'])
    assert not inspect(b)['visible'] and inspect(a)['visible'] and not inspect(c)['visible']
    passed('individual close/open overrides the stash snapshot safely during animation')
    # Exercise each real screen independently and restore each window's own frame.
    for screen,target in zip(screens,targets):
        move(*target);key();wait(a,lambda s:not s['visible']);time.sleep(.15)
        v=screen['visible'];top=main_height-v[1]-v[3]
        def on_screen(state):
            r=state['stash']['indicator']
            return state['stash']['tip_visible'] and v[0]<=r['x'] and r['x']+r['width']<=v[0]+v[2] and top<=r['y'] and r['y']+r['height']<=top+v[3]
        r=wait(a,on_screen)['stash']['indicator']
        assert v[0]<=r['x'] and r['x']+r['width']<=v[0]+v[2] and top<=r['y'] and r['y']+r['height']<=top+v[3],(v,r)
        key();wait(a,lambda s:s['visible'] and not s['stash']['active']);assert inspect(a)['position']==before[a]['position']
    passed(f'{len(targets)} actual screens: cursor screen selection and original position restore')
    report['result']='passed'
finally:
    if 'PID' in globals():
        try:
            if inspect(a)['stash']['active']:key();time.sleep(.6)
        except Exception:pass
    for id in ids:
        try:cli('close',id)
        except subprocess.CalledProcessError:pass
    move(pointer['x'],pointer['y'])
    save('report',report)
    print('Evidence: '+str(OUT),flush=True)
