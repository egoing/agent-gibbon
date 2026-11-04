#!/usr/bin/env python3
"""PID-bound real tray + capture selection / image stash regression on macOS."""
import json,subprocess,tempfile,time,uuid,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];CLI=ROOT/'scripts/agent-gibbon-dev'
INFO=json.loads(subprocess.check_output(['node',str(ROOT/'scripts/dev.mjs'),'info'],text=True))
ID='stash-capture-'+uuid.uuid4().hex[:8];OUT=ROOT/'docs/verification'/ID;OUT.mkdir(parents=True)
REPORT={'instance':INFO,'id':ID,'checks':[]};created=[ID]
def run(*args):return subprocess.check_output(list(map(str,args)),text=True)
def cli(action,id=None,*args):
    d=json.loads(run(CLI,action,*(['--id',id] if id else []),*args));assert d['ok'],d
    if action=='inspect':assert d['instance']==INFO['id']
    return d
def wait(test):
    until=time.monotonic()+10
    while time.monotonic()<until:
        r=test()
        if r:return r
        time.sleep(.1)
    raise AssertionError('timed out')
def check(s):REPORT['checks'].append(s);print(s,flush=True)
with tempfile.TemporaryDirectory(prefix='gibbon-stash-capture-') as tmp:
    tmp=Path(tmp)
    for src,name in [('macos-input.swift','mouse'),('entrance-pointer.swift','pointer'),('auto-layout-native.swift','native'),('stash-key.swift','key'),('design-system-ax.swift','ax')]:
        subprocess.run(['swiftc',str(ROOT/'scripts'/src),'-o',str(tmp/name)],check=True)
    pointer=json.loads(run(tmp/'pointer'))
    cli('open',ID,'--markdown','# 캡처 복원 검증\n'+ID)
    expected=INFO['binary']+' --serve'
    rows=run('ps','-axo','pid=,command=').splitlines();PID=next(int(r.strip().split(maxsplit=1)[0]) for r in rows if r.strip().split(maxsplit=1)[-1]==expected)
    REPORT['pid']=PID;REPORT['executable']=expected
    def native():return json.loads(run(tmp/'native',PID))
    def key():run(tmp/'key')
    def inspect():return cli('inspect',ID)
    def select(label):
        prefix=f'tell application "System Events" to tell (first application process whose unix id is {PID}) to '
        frame=run('osascript','-e',prefix+'get {position, size} of menu bar item 1 of menu bar 2')
        x,y,w,h=map(float,frame.split(','));run(tmp/'pointer','move',x+w/2,y+h/2);run(tmp/'mouse','click',x+w/2,y+h/2);time.sleep(.25)
        run('osascript','-e',prefix+'click menu item '+json.dumps(label,ensure_ascii=False)+' of menu 1 of menu bar item 1 of menu bar 2')
    try:
        assert inspect()['stash']['shortcut']=='Ctrl+Alt+Cmd'
        time.sleep(1.1);run(tmp/'pointer','move',600,500);wait(lambda:not inspect()['animating'])
        before=inspect();assert any(ID in w.get('kCGWindowName','') for w in native()['windows'])
        select('전체 창 접어두기 / 복원 · Ctrl+Alt+Cmd');wait(lambda:not inspect()['visible'])
        select('전체 창 접어두기 / 복원 · Ctrl+Alt+Cmd');wait(lambda:inspect()['visible'] and not inspect()['stash']['active'])
        assert inspect()['position']==before['position'] and inspect()['size']==before['size']
        check('actual PID-specific tray item toggles and restores exact geometry')
        state=inspect();title='Agent Gibbon ['+state['dev_label']+'] — '+ID
        run(tmp/'ax',PID,title,'position',500,350);time.sleep(.2);before=inspect()
        select('영역 캡처 · Ctrl+Shift+1')
        def overlays():return [w for w in native()['windows'] if '영역 캡처' in w.get('kCGWindowName','')]
        original=wait(overlays);REPORT['overlays_before']=original
        key();wait(lambda:not inspect()['stash']['animating'] and inspect()['stash']['active']);assert not overlays()
        key();wait(lambda:not inspect()['stash']['active'])
        expected_bounds={w['kCGWindowNumber']:w['kCGWindowBounds'] for w in original}
        def settled_overlays():
            rows=overlays()
            return rows if {w['kCGWindowNumber']:w['kCGWindowBounds'] for w in rows}==expected_bounds else None
        restored=wait(settled_overlays)
        REPORT['overlays_restored']=restored
        assert {w['kCGWindowNumber']:w['kCGWindowBounds'] for w in original}=={w['kCGWindowNumber']:w['kCGWindowBounds'] for w in restored}
        check('all capture overlays hide/restore with identical native IDs and full-screen bounds')
        # End selection explicitly; creating a new capture is separate from
        # this visibility regression. Foreground input is needed for Escape.
        front=run('osascript','-e','tell application "System Events" to get unix id of first application process whose frontmost is true').strip()
        run('osascript','-e',f'tell application "System Events" to set frontmost of (first application process whose unix id is {PID}) to true')
        primary=next(w for w in restored if w['kCGWindowBounds']['X']==0 and w['kCGWindowBounds']['Y']==0)
        run(tmp/'pointer','move',100,400);run(tmp/'mouse','click',100,400)
        run('osascript','-e','tell application "System Events" to key code 53')
        wait(lambda:not overlays())
        if front!=str(PID):
            run('osascript','-e',f'tell application "System Events" to set frontmost of (first application process whose unix id is {front}) to true')
        # Optional known test image: never select a user's existing image implicitly.
        if len(sys.argv)>1:
            image=sys.argv[1]
            assert cli('inspect',image)['note']['capture'], 'Expected a pre-created test capture'
            cli('open',image);created.append(image);REPORT['image_id']=image
            before_image=wait(lambda:(v if (v:=cli('inspect',image))['visible'] else None))
            key();wait(lambda:not cli('inspect',image)['visible']);key();wait(lambda:cli('inspect',image)['visible'] and not inspect()['stash']['active'])
            after=cli('inspect',image)
            assert before_image['position']==after['position'] and before_image['size']==after['size'] and before_image['note']['capture']==after['note']['capture']
            REPORT['image_before']=before_image;REPORT['image_after']=after
            check('real captured image retains bounds, aspect ratio and content across stash')
        REPORT['result']='passed'
    finally:
        if inspect()['stash']['active']:key();time.sleep(.6)
        for id in created:cli('close',id)
        run(tmp/'pointer','move',pointer['x'],pointer['y'])
        (OUT/'report.json').write_text(json.dumps(REPORT,ensure_ascii=False,indent=2))
        print('Evidence: '+str(OUT),flush=True)
