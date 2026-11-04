#!/usr/bin/env python3
"""macOS live layout regression. Uses only this checkout's CLI and PID-bound AX.
Requires dev:build and macOS event posting/accessibility permissions.
Only creates/operates auto-proof-* test notes; does not stop any app.
"""
import json, subprocess, time, tempfile, uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
INFO=json.loads(subprocess.check_output(['node',str(ROOT/'scripts/dev.mjs'),'info'],text=True))
CLI=str(ROOT/'scripts/agent-gibbon-dev')
LOG={'origin':INFO,'checks':[],'states':[]}
FRONT=subprocess.check_output(['osascript','-e','tell application "System Events" to get unix id of first application process whose frontmost is true'],text=True).strip()
PREFIX='auto-proof-'+uuid.uuid4().hex[:6]
IDS=[]
def run(args): return subprocess.check_output(list(map(str,args)),text=True)
def cli(action,note,*args):
    r=json.loads(run([CLI,action,'--id',note,*args])); assert r['ok'],r
    if action=='inspect': assert r['instance']==INFO['id'] and r['id']==note,r
    return r
def inspect(note): return cli('inspect',note)
def frame(s): return (s['position']['x'],s['position']['y'],s['size']['width'],s['size']['height'])
def check(name): LOG['checks'].append(name); print('PASS:',name,flush=True)
def wait(note):
    for _ in range(60):
        s=inspect(note)
        if not s['animating']: return s
        time.sleep(.1)
    raise AssertionError(s)
def overlap(a,b):
    x,y,w,h=frame(a); X,Y,W,H=frame(b)
    return x<X+W and X<x+w and y<Y+H and Y<y+h
with tempfile.TemporaryDirectory(prefix='gibbon-auto-') as folder:
    helper=Path(folder)/'ax'; pointer=Path(folder)/'pointer'; mouse=Path(folder)/'mouse'
    for src,out in [('design-system-ax.swift',helper),('entrance-pointer.swift',pointer),('macos-input.swift',mouse)]:
        subprocess.run(['swiftc',str(ROOT/'scripts'/src),'-o',str(out)],check=True)
    original=json.loads(run([pointer])); scale=original['scale']
    def move(x,y): run([pointer,'move',x,y])
    def open_note(index):
        note=f'{PREFIX}-{index:02d}'; IDS.append(note)
        move(600,650)
        s=cli('open',note,'--title',f'자동배치 검증 {index}','--markdown',f'# {PREFIX}-{index}\n'+'본문\n\n'*(index*4+1))
        assert not s['auto_layout_excluded'],s
        move(650,650); return note
    def ui(note,action,*args,feedback=False):
        state=inspect(note)
        title=f"Agent Gibbon [{state['dev_label']}] — {note}"+(' · Feedback' if feedback else '')
        for _ in range(30):
            p=subprocess.run([str(helper),str(pid),title,action,*args],text=True,capture_output=True)
            if p.returncode==0: return p.stdout
            time.sleep(.1)
        raise AssertionError(p.stderr)
    try:
        first=open_note(1); wait(first)
        rows=run(['ps','-axo','pid=,comm=']).splitlines()
        pids=[int(row.strip().split(None,1)[0]) for row in rows if row.strip().split(None,1)[-1]==INFO['binary']]
        assert len(pids)==1,pids
        pid=pids[0]; LOG['pid']=pid
        assert PREFIX in ui(first,'dump')
        check('CLI instance, binary PID, native window title and unique body marker')
        previous=inspect(first)
        samples=[]
        for index in range(2,7):
            note=open_note(index)
            for _ in range(3):
                samples.append(inspect(first)); time.sleep(.04)
            wait(note)
            for n in IDS: wait(n)
        states=[inspect(n) for n in IDS]; LOG['states'].append({'ordinary':states}); LOG['samples']=samples
        assert all(not s['auto_layout_excluded'] and not s['focused'] for s in states),states
        assert all(s['size']['width']==300*scale and 48*scale<=s['size']['height']<=s['content_height_cap']*scale for s in states)
        for i,s in enumerate(states):
            for other in states[:i]: assert not overlap(s,other),(s,other)
        columns={}
        for s in states: columns.setdefault(s['position']['x'],[]).append(s)
        assert len(columns)>=2,columns
        for column in columns.values():
            column.sort(key=lambda s:s['position']['y'])
            for a,b in zip(column,column[1:]): assert a['position']['y']+a['size']['height']==b['position']['y'],(a,b)
        xs=sorted(columns)
        assert all(b-a==300*scale for a,b in zip(xs,xs[1:])),xs
        assert any(s['animating'] for s in samples),samples
        check('ordinary sequential opens: 300px, bounded heights, gapless rows/columns, transition intermediate frames, focus preserved')
        before=[frame(s) for s in states]
        cli('update',first,'--markdown','# 更新\n본문 변경만 수행')
        time.sleep(.6)
        assert [frame(inspect(n)) for n in IDS]==before
        check('visible content update does not reflow or resize')
        # Actual native drag, not a programmatic AX geometry write.
        s=inspect(first); x,y,w,h=[v/scale for v in frame(s)]
        move(x+40,y+60); run([mouse,'drag',x+40,y+60,900,260]); time.sleep(.2)
        dragged=inspect(first)
        assert dragged['auto_layout_excluded'] and frame(dragged)!=frame(s),(s,dragged)
        saved=frame(dragged)
        open_note(7); wait(IDS[-1]); assert frame(inspect(first))==saved
        assert all(not overlap(inspect(first),inspect(n)) for n in IDS[1:])
        check('native drag excludes only its note; new open avoids and preserves obstacle')
        # Begin editor via PID/title AX, then use actual editor save path.
        ui(first,'click','메모 편집'); time.sleep(.3)
        assert not inspect(first)['auto_layout_excluded']
        ui(first,'draft','# 편집으로 자동배치 복귀\n'+PREFIX)
        ui(first,'click','저장',feedback=True); time.sleep(.4)
        assert not inspect(first)['auto_layout_excluded'] and frame(inspect(first))==saved
        open_note(8)
        movement=[inspect(first) for _ in range(4)]
        LOG['movement']=movement
        wait(IDS[-1]); wait(first)
        assert any(s['animating'] for s in movement) and len({frame(s) for s in movement})>1, movement
        assert frame(inspect(first))!=saved
        check('editor action/save re-includes without immediate layout; next open reflows it')
        s=inspect(first); x,y,w,h=[v/scale for v in frame(s)]
        move(x+1,y+h/2); time.sleep(.15); run([mouse,'drag',x+1,y+h/2,x+61,y+h/2]); time.sleep(.2)
        resized=inspect(first)
        assert resized['auto_layout_excluded'] and resized['size']!=s['size'],(s,resized)
        saved=frame(resized); time.sleep(.5); assert frame(inspect(first))==saved
        cli('update',first,'--markdown','# CLI 내용 갱신으로 복귀')
        assert not inspect(first)['auto_layout_excluded'] and frame(inspect(first))==saved
        check('native resize excludes; polling preserves flag/size; explicit content update re-includes')
        hidden=IDS[-1]; cli('close',hidden)
        hidden_frame=frame(inspect(hidden)); cli('update',hidden,'--markdown','숨김 갱신')
        assert not inspect(hidden)['visible'] and frame(inspect(hidden))==hidden_frame
        move(600,650); reopened=cli('open',hidden); move(650,650); wait(hidden); wait(first)
        assert inspect(hidden)['visible'] and inspect(first)['size']['width']==300*scale
        check('hidden update preserves visibility/geometry; ordinary reopen reflows all eligible notes')
        # Two quick opens retarget active flights; check final reservations do not overlap.
        open_note(9); open_note(10)
        for n in IDS: wait(n)
        states=[inspect(n) for n in IDS]
        for i,s in enumerate(states):
            for other in states[:i]: assert not overlap(s,other),(s,other)
        LOG['states'].append({'final':states})
        check('rapid opens replace active animation destinations without final overlap')
    finally:
        Path('/tmp/gibbon-auto-layout-live.json').write_text(json.dumps(LOG,ensure_ascii=False,indent=2))
        for note in IDS: cli('close',note)
        move(original['x'],original['y'])
        subprocess.run(['osascript','-e',f'tell application "System Events" to set frontmost of (first application process whose unix id is {FRONT}) to true'],capture_output=True)
