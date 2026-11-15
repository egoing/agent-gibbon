#!/usr/bin/env python3
"""PID/title-bound native macOS smoke; only this worktree's test memos."""
import json
import os
import ctypes
from pathlib import Path
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
OUT = Path(os.environ.get('GIBBON_RELATION_SMOKE_OUTPUT','/tmp/gibbon168-native'))
OUT.mkdir(exist_ok=True)
def run(*args):
    return subprocess.check_output(list(map(str,args)),cwd=ROOT,text=True,timeout=30).strip()
for source, name in [('design-system-ax.swift','ax'),('auto-layout-native.swift','native'),('macos-input.swift','input')]:
    run('swiftc', ROOT/'scripts'/source, '-o', OUT/name)
info=json.loads(run('node','scripts/dev.mjs','info'))
assert info['root']==str(ROOT)
ids=['relation-'+uuid.uuid4().hex[:8]+suffix for suffix in ['-a','-b']]
records=[]
front=run('swift',ROOT/'scripts/frontmost.swift')
created=[]
config=Path('/tmp')/f"floatnote-{os.getuid()}-{info['id']}"/'relation-prototype.json'
original=config.read_bytes() if config.exists() else b'[]'
def restore_relations():
    import tempfile
    fd,name=tempfile.mkstemp(dir=config.parent)
    with os.fdopen(fd,'wb') as f:f.write(original)
    os.replace(name,config)
def cli(action,id):
    args=[ROOT/'scripts/agent-gibbon-dev',action,'--id',id]
    if action=='open': args+=['--markdown','관계 검증 '+id]
    value=json.loads(run(*args)); assert value['ok'],value
    if action=='inspect': assert value['instance']==info['id'] and value['id']==id
    return value
def native(): return json.loads(run(OUT/'native',pid))
def overlays(): return [w for w in native()['windows'] if 'relation-prototype-' in w.get('kCGWindowName','')]
def ax(index,action,*args): return run(OUT/'ax',pid,titles[index],action,*args)
def snapshot(case):
    time.sleep(.35)
    record={'case':case,'notes':[cli('inspect',id) for id in ids], 'native':native()}
    records.append(record)
    return record
try:
    for id in ids:
        opened=cli('open',id);created.append(id)
    lib=ctypes.CDLL('/usr/lib/libproc.dylib')
    matches=[]
    for line in run('ps','-axo','pid=').splitlines():
        candidate=int(line);buf=ctypes.create_string_buffer(4096)
        if lib.proc_pidpath(candidate,buf,len(buf))>0 and buf.value.decode()==info['binary']:matches.append(candidate)
    assert len(matches)==1,matches
    pid=int(matches[0]);titles=[f"Agent Gibbon [{opened['dev_label']}] — {id}" for id in ids]
    for index,id in enumerate(ids):
        state=cli('inspect',id); assert state['note']['markdown']=='관계 검증 '+id
        ax(index,'position',80+index*580,160+index*100)
    time.sleep(2)
    for i in range(2): ax(i,'resize',300,180)
    before=snapshot('before-relation')
    run('python3','scripts/relation-prototype.py','--clear');time.sleep(.4)
    assert not overlays()
    run('python3','scripts/relation-prototype.py',*ids,'--label','선행 작업')
    linked=snapshot('linked')
    assert len(overlays())==1,linked
    assert [s['focused'] for s in linked['notes']]==[s['focused'] for s in before['notes']], 'relation creation changed memo focus'
    first=overlays()[0]['kCGWindowBounds']
    ax(1,'position',700,310)
    moved=snapshot('moved'); assert overlays()[0]['kCGWindowBounds']==first, 'small movement resized the reserved overlay'
    ax(1,'resize',330,240);snapshot('resized')
    run('python3',ROOT/'scripts/relation-endpoint-pixels.py',pid,*ids,'--native',OUT/'native','--output',OUT/'endpoints-resize.png')
    for x,y in [(80,600),(700,310)]:
        ax(1,'position',x,y);snapshot('port-direction-change');assert len(overlays())==1
        run('python3',ROOT/'scripts/relation-endpoint-pixels.py',pid,*ids,'--native',OUT/'native','--output',OUT/f'endpoints-{x}-{y}.png')
    run('python3','scripts/relation-prototype.py',ids[1],ids[0],'--label','선행 작업');snapshot('reverse-identity');assert len(overlays())==1
    run('python3','scripts/relation-prototype.py',*ids,'--label','선행 작업');snapshot('restore-identity');assert len(overlays())==1
    # The overlay bounding rectangle overlaps the right edge of source memo.
    bounds=overlays()[0]['kCGWindowBounds']
    source=json.loads(ax(0,'geometry'))['window']
    x=source['x']+source['width']-20;y=source['y']+source['height']/2
    assert bounds['X']<=x<=bounds['X']+bounds['Width'] and bounds['Y']<=y<=bounds['Y']+bounds['Height'],(bounds,source)
    run(OUT/'input','click',x,y)
    clicked=snapshot('native-click-through')
    assert clicked['notes'][0]['focused'],clicked
    ax(0,'click','메모 접기')
    folded=snapshot('folded');assert folded['notes'][0]['folded']
    ax(0,'click','메모 펼치기');snapshot('unfolded')
    cli('close',ids[1]);hidden=snapshot('hidden');assert not overlays()
    cli('open',ids[1]);time.sleep(1.5)
    ax(1,'position',700,310);restored=snapshot('restored');assert len(overlays())==1
    overlay=overlays()[0]
    run('screencapture','-x','-o','-l',overlay['kCGWindowNumber'],OUT/'relation-native.png')
    for index,title in enumerate(titles):
        window=next(w for w in native()['windows'] if w.get('kCGWindowName')==title)
        run('screencapture','-x','-o','-l',window['kCGWindowNumber'],OUT/f'memo-{index}.png')
    ax(1,'position',80,160);snapshot('overlap');assert not overlays()
    ax(1,'position',700,310);snapshot('separated');assert len(overlays())==1
    ax(1,'click','메모 창 닫기');snapshot('native-closed');assert not overlays()
    cli('open',ids[1]);time.sleep(1.5);ax(1,'position',700,310);snapshot('recreated');assert len(overlays())==1
    run('python3','scripts/relation-prototype.py','--clear');snapshot('cleared');assert not overlays()
    print(json.dumps({'ok':True,'root':str(ROOT),'instance':info['id'],'binary':info['binary'],'pid':pid,'ids':ids,'titles':titles,'cases':[r['case'] for r in records]},ensure_ascii=False))
finally:
    restore_relations()
    for id in created:
        subprocess.run([str(ROOT/'scripts/agent-gibbon-dev'),'close','--id',id],cwd=ROOT,capture_output=True)
    if run('swift',ROOT/'scripts/frontmost.swift')!=front:
        run('swift','-e','import AppKit; let pid=pid_t(CommandLine.arguments.last!)!; if let app=NSRunningApplication(processIdentifier:pid) { app.activate(options:[]) }',front)
        time.sleep(.3)
    front_after=run('swift',ROOT/'scripts/frontmost.swift')
    assert front_after==front,(front,front_after)
    (OUT/'evidence.json').write_text(json.dumps({'info':info,'front_before':front,'front_after':front_after,'records':records},ensure_ascii=False,indent=2))
