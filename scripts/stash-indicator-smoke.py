#!/usr/bin/env python3
"""Actual macOS indicator appearance, flight, timing and focus regression."""
import json, subprocess, tempfile, time, uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
INFO=json.loads(subprocess.check_output(['node',str(ROOT/'scripts/dev.mjs'),'info']))
ID='stash-indicator-'+uuid.uuid4().hex[:8]
OUT=ROOT/'docs/verification'/ID;OUT.mkdir()
report={'instance':INFO,'id':ID,'checks':[]}
def run(*args):return subprocess.check_output(list(map(str,args)),text=True)
def cli(action):
    r=json.loads(run(ROOT/'scripts/agent-gibbon-dev',action,'--id',ID));assert r['ok'] and r['instance']==INFO['id'];return r
def wait(predicate):
    end=time.monotonic()+10
    while time.monotonic()<end:
        s=cli('inspect')['stash']
        if predicate(s):return s
        time.sleep(.03)
    raise AssertionError(s)
def save(name,value):(OUT/(name+'.json')).write_text(json.dumps(value,ensure_ascii=False,indent=2))
with tempfile.TemporaryDirectory() as tmp:
    tmp=Path(tmp)
    for src,name in [('entrance-pointer.swift','pointer'),('auto-layout-native.swift','native'),('stash-key.swift','key'),('design-system-ax.swift','ax'),('frontmost.swift','front')]:
        subprocess.run(['swiftc',str(ROOT/'scripts'/src),'-o',str(tmp/name)],check=True)
    def helper(name,*args):return run(tmp/name,*args)
    pointer=json.loads(helper('pointer'))
    r=json.loads(run(ROOT/'scripts/agent-gibbon-dev','open','--id',ID,'--markdown',ID));assert r['instance']==INFO['id']
    expected=INFO['binary']+' --serve'
    pid=next(int(p[0]) for l in run('ps','-axo','pid=,command=').splitlines() if len(p:=l.strip().split(maxsplit=1))==2 and p[1]==expected)
    report.update(pid=pid,executable=expected)
    title='Agent Gibbon ['+r['dev_label']+'] — stash-indicator'
    def native():return json.loads(helper('native',pid))
    def shot(name):
        w=next(w for w in native()['windows'] if w.get('kCGWindowName')==title)
        assert w['kCGWindowOwnerPID']==pid;save(name+'-source',w)
        subprocess.run(['screencapture','-x','-o','-l',str(w['kCGWindowNumber']),str(OUT/(name+'.png'))],check=True)
    try:
        if cli('inspect')['stash']['active']:helper('key');wait(lambda s:not s['active'])
        time.sleep(1.2);helper('pointer','move',600,500);time.sleep(.6)
        focus=helper('front');helper('key')
        initial=wait(lambda s:s['active'] and not s['animating'] and s['tip_visible'])
        time.sleep(.15);shot('initial');r=cli('inspect')['stash']['indicator']
        text=helper('ax',pid,title,'dump');assert '복원:' in text and 'Ctrl Opt ⌘' in text
        assert helper('front')==focus
        report['checks'].append('initial settled bubble + mascot, actual key, unchanged frontmost PID')
        time.sleep(6.1);helper('pointer','move',r['x']+r['width']-24,r['y']+24)
        wait(lambda s:s['indicator_moving']);frames=[]
        for _ in range(7):
            s=cli('inspect')['stash'];frames.append(s)
            if s['indicator_moving']:assert not s['tip_visible']
            time.sleep(.025)
        end=wait(lambda s:not s['indicator_moving'] and s['tip_visible']);save('flight',frames)
        y=end['indicator']['y'];assert any(min(y,r['y'])<s['indicator']['y']<max(y,r['y']) for s in frames)
        time.sleep(.15);shot('arrived');assert helper('front')==focus
        helper('pointer','move',end['indicator']['x']+end['indicator']['width']-24,y+24)
        wait(lambda s:s['indicator_moving'] and not s['tip_visible'])
        helper('pointer','move',600,500)
        wait(lambda s:not s['indicator_moving'] and s['tip_visible'])
        report['checks'].append('intermediate eased flight positions, no bubble during flight, bubble after arrival, immediate reentry')
        helper('pointer','move',600,500);helper('key');wait(lambda s:not s['active'])
        assert cli('inspect')['visible'];report['checks'].append('same shortcut restores note')
        report['result']='passed'
    finally:
        if cli('inspect')['stash']['active']:helper('key');time.sleep(.6)
        cli('close');helper('pointer','move',pointer['x'],pointer['y']);save('report',report)
        print(OUT)
