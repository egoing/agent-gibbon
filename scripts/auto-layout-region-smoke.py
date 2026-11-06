#!/usr/bin/env python3
"""Native menu/region selection and content-cap regression in this dev instance.
Uses a unique prefix and only hides notes created by this run. Does not quit apps.
"""
import json, subprocess, tempfile, time, uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; CLI=ROOT/'scripts/agent-gibbon-dev'
INFO=json.loads(subprocess.check_output(['node',str(ROOT/'scripts/dev.mjs'),'info'],text=True))
FRONT=subprocess.check_output(['osascript','-e','tell application "System Events" to get unix id of first application process whose frontmost is true'],text=True).strip()
PREFIX='region-proof-'+uuid.uuid4().hex[:6]; created=[]; LOG={'origin':INFO,'checks':[]}
def run(args):return subprocess.check_output(list(map(str,args)),text=True)
def cli(action,note=None,*args):
    v=json.loads(run([CLI,action]+(['--id',note] if note else [])+list(args)));assert v['ok'],v
    if action=='inspect':assert v['instance']==INFO['id'],v
    return v
def inspect(n):return cli('inspect',n)
def frame(v):return (v['position'],v['size'])
def check(s):LOG['checks'].append(s);print('PASS:',s,flush=True)
def apple(script):return run(['osascript','-e',script]).strip()
with tempfile.TemporaryDirectory(prefix='gibbon-regions-') as folder:
    folder=Path(folder)
    for source,name in [('design-system-ax.swift','ax'),('entrance-pointer.swift','pointer'),('macos-input.swift','mouse'),('auto-layout-native.swift','native')]:
        subprocess.run(['swiftc',str(ROOT/'scripts'/source),'-o',str(folder/name)],check=True)
    original=json.loads(run([folder/'pointer']));scale=original['scale']
    def move(x,y):run([folder/'pointer','move',x,y])
    def wait(n):
        for _ in range(60):
            v=inspect(n)
            if v['visible'] and not v['animating']:return v
            time.sleep(.1)
        raise AssertionError(v)
    def open_note(suffix,body):
        n=PREFIX+'-'+suffix;created.append(n);move(600,650)
        cli('open',n,'--markdown',body);move(650,650);wait(n);return n
    def ui(n,label,feedback=False,*args):
        s=inspect(n);title=f"Agent Gibbon [{s['dev_label']}] — {n}"+(' · Feedback' if feedback else '')
        command='draft' if label=='draft' else 'click'
        for _ in range(30):
            r=subprocess.run([str(folder/'ax'),str(pid),title,command,*((list(args)) if command=='draft' else [label])],capture_output=True,text=True)
            if r.returncode==0:return r.stdout
            time.sleep(.1)
        raise AssertionError(r.stderr)
    def select(mode,x,y,w,h):
        before={n['id'] for n in cli('list')['windows']}
        prefix=f'tell application "System Events" to tell (first application process whose unix id is {pid}) to '
        a,b,c,d=map(float,apple(prefix+'get {position, size} of menu bar item 1 of menu bar 2').split(','))
        run([folder/'mouse','click',a+c/2,b+d/2]);time.sleep(.15)
        label='영역 캡처\tCtrl ⇧ 1' if mode=='image' else '영역 텍스트 메모\tCtrl ⇧ 2'
        apple(prefix+'click menu item '+json.dumps(label,ensure_ascii=False)+' of menu 1 of menu bar item 1 of menu bar 2')
        time.sleep(1)
        native=json.loads(run([folder/'native',pid]));LOG.setdefault('overlays',[]).append(native)
        assert any('capture' in w.get('kCGWindowName','').lower() or '영역' in w.get('kCGWindowName','') for w in native['windows']),native
        move(x,y);run([folder/'mouse','drag',x,y,x+w,y+h])
        for _ in range(60):
            fresh={n['id'] for n in cli('list')['windows']}-before
            if fresh:break
            time.sleep(.1)
        assert len(fresh)==1,fresh
        n=fresh.pop();created.append(n);v=wait(n)
        assert v['auto_layout_excluded'] and not v['animating'],v
        assert all(abs(a-b)<=2 for a,b in zip([v['position']['x']/scale,v['position']['y']/scale,v['size']['width']/scale,v['size']['height']/scale],[x,y,w,h])),v
        return n
    try:
        short=open_note('01-short','짧은 본문')
        pidrows=[r.strip().split(None,1) for r in run(['ps','-axo','pid=,comm=']).splitlines()]
        pid=next(int(p) for p,c in pidrows if c==INFO['binary']);LOG['pid']=pid
        s=inspect(short);assert s['size']['height']==48*scale and s['content_height_cap']==48,s
        long=open_note('02-long','# 긴 메모\n'+'충분한 본문\n\n'*100)
        s=inspect(long);assert s['size']['height']<=420*scale and s['content_height_cap']==420,s
        assert inspect(short)['size']['height']==48*scale
        check('short content stays at 48px; long content never exceeds 420px')
        baseline={n:frame(inspect(n)) for n in [short,long]}
        text=select('text',1000,200,240,120)
        assert {n:frame(inspect(n)) for n in baseline}==baseline
        LOG['text_initial']=inspect(text)
        check('actual region text selection keeps exact bounds, starts excluded despite automatic editor, does not reflow')
        image=select('image',1800,150,160,100)
        assert {n:frame(inspect(n)) for n in baseline}==baseline
        LOG['image_initial']=inspect(image)
        check('actual image selection keeps bounds and aspect ratio, starts excluded, does not reflow')
        saved={n:frame(inspect(n)) for n in [text,image]}
        extra=open_note('03-new','새 일반 메모')
        assert {n:frame(inspect(n)) for n in saved}==saved
        check('new normal open avoids both region obstacles without changing their frames')
        ui(text,'draft',False,'# 영역 편집 후 복귀\n'+PREFIX)
        ui(text,'저장',True);time.sleep(.4)
        assert not inspect(text)['auto_layout_excluded'] and frame(inspect(text))==saved[text]
        ui(image,'상단 고정');time.sleep(.2)
        assert not inspect(image)['auto_layout_excluded']
        open_note('04-reflow','다음 배치')
        wait(text);wait(image)
        image_state=inspect(image);w=image_state['size']['width']/scale;h=image_state['size']['height']/scale
        assert abs(w/h-1.6)<.02 and w<=160 and h<=100,image_state
        assert inspect(text)['size']['width']==300*scale
        LOG['image_after']=image_state;LOG['text_after']=inspect(text)
        check('region edit/pin action re-includes; next normal open reflows text and preserves image ratio without enlarging original')
        # A real native menu reopen takes the direct presentation path.
        restore_title='일반 메모 복원 '+PREFIX
        cli('update',extra,'--title',restore_title)
        cli('close',extra)
        prefix=f'tell application "System Events" to tell (first application process whose unix id is {pid}) to '
        a,b,c,d=map(float,apple(prefix+'get {position, size} of menu bar item 1 of menu bar 2').split(','))
        run([folder/'mouse','click',a+c/2,b+d/2]);time.sleep(.15)
        apple(prefix+'click menu item '+json.dumps(restore_title[:30])+' of menu 1 of menu bar item 1 of menu bar 2')
        restored=wait(extra);assert not restored['focused']
        check('native tray hidden-note restore uses direct layout and preserves focus')
    finally:
        Path('/tmp/gibbon-auto-regions-live.json').write_text(json.dumps(LOG,ensure_ascii=False,indent=2))
        for n in created:cli('close',n)
        move(original['x'],original['y'])
        subprocess.run(['osascript','-e',f'tell application "System Events" to set frontmost of (first application process whose unix id is {FRONT}) to true'],capture_output=True)
