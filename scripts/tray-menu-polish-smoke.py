#!/usr/bin/env python3
"""macOS: PID-bound #76 native menu regression. Uses only this worktree's CLI.
Creates its own notes, leaves them hidden, and restores pointer/frontmost app.
Requires the built dev app and a baseline of fewer than 30 saved entries.
"""
import json, subprocess, tempfile, time, uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
CLI=ROOT/'scripts/agent-gibbon-dev'
OUT=ROOT/'docs/verification/tray-menu-polish-20261106'
OUT.mkdir(parents=True,exist_ok=True)
def run(*args): return subprocess.check_output(list(map(str,args)),text=True,timeout=30).strip()
INFO=json.loads(run('node',ROOT/'scripts/dev.mjs','info'))
PREFIX='tray76-'+uuid.uuid4().hex[:6]
REPORT={'origin':INFO,'prefix':PREFIX,'checks':[]}
created=[]
def cli(action,id=None,*args):
    value=json.loads(run(CLI,action,*(['--id',id] if id else []),*args))
    assert value['ok'],value
    if action=='inspect': assert value['instance']==INFO['id'] and value['id']==id,value
    return value
def wait(test):
    until=time.monotonic()+10
    while time.monotonic()<until:
        value=test()
        if value:return value
        time.sleep(.1)
    raise AssertionError('condition timed out')
def check(text): REPORT['checks'].append(text); print('PASS: '+text,flush=True)
def apple(script): return run('osascript','-e',script)
with tempfile.TemporaryDirectory(prefix='gibbon-tray76-') as temp:
    temp=Path(temp)
    for src,name in [('tray-menu-ax.swift','menu'),('macos-input.swift','mouse'),('entrance-pointer.swift','pointer'),('auto-layout-native.swift','native')]:
        subprocess.run(['swiftc',str(ROOT/'scripts'/src),'-o',str(temp/name)],check=True)
    original=json.loads(run(temp/'pointer'))
    front=apple('tell application "System Events" to get unix id of first application process whose frontmost is true')
    proof=PREFIX+'-proof';created.append(proof)
    cli('open',proof,'--title','최근검증 기준','--markdown','# '+PREFIX)
    state=cli('inspect',proof);assert PREFIX in state['note']['markdown']
    rows=[r.strip().split(None,1) for r in run('ps','-axo','pid=,comm=').splitlines()]
    pids=[int(pid) for pid,path in rows if path==INFO['binary']]
    assert len(pids)==1,pids
    pid=pids[0];REPORT.update(pid=pid,executable=INFO['binary'],proof=state)
    def native(): return json.loads(run(temp/'native',pid))
    wait(lambda:any(proof in w.get('kCGWindowName','') for w in native()['windows']))
    REPORT['native_origin']=native()
    check('CLI instance, unique body, exact executable PID and native window title agree')
    prefix=f'tell application "System Events" to tell (first application process whose unix id is {pid}) to '
    x,y,w,h=map(float,apple(prefix+'get {position, size} of menu bar item 1 of menu bar 2').split(','))
    def open_menu():
        run(temp/'pointer','move',x-100,y+100)
        run(temp/'pointer','move',x+w/2,y+h/2)
        time.sleep(.15)
        run(temp/'mouse','click',x+w/2,y+h/2)
        time.sleep(.15)
        rows=json.loads(run(temp/'menu',pid))
        first_separator=next(i for i,row in enumerate(rows) if not row['title'])
        assert all(row['title']!='영역 에이전트 터미널' for row in rows[:first_separator]),rows
        return rows
    def choose(title): run(temp/'menu',pid,title);time.sleep(.15)
    def entries(menu):
        separators=[i for i,row in enumerate(menu) if not row['title']]
        assert len(separators)==2,menu
        return menu[separators[0]+1:separators[1]]
    def snapshot(name):
        windows=[w for w in native()['windows'] if w.get('kCGWindowLayer')==101]
        assert windows,'No PID-owned menu window'
        rect=windows[0]['kCGWindowBounds']; REPORT.setdefault('captures',{})[name]=rect
        run('screencapture','-x','-R'+','.join(str(int(rect[k])) for k in ['X','Y','Width','Height']),OUT/(name+'.png'))
    try:
        cli('close',proof)
        menu=open_menu();assert menu[0]['title']==INFO['name'] and menu[1]['title']=='메모 관리'
        assert all(row['enabled'] for row in entries(menu))
        assert any(row['title']=='영역 캡처\tCtrl ⇧ 1' for row in menu)
        cli('inspect',proof);cli('update',proof,'--status','메뉴 열린 중 갱신')
        snapshot('menu-background-update')
        choose('최근검증 기준');wait(lambda:cli('inspect',proof)['visible'])
        check('hidden item is enabled and restores after inspect/update while menu stays open')
        baseline=cli('list')['windows'];assert len(baseline)<=30, len(baseline)
        for index in range(30-len(baseline)):
            id=f'{PREFIX}-{index:02}';created.append(id)
            cli('open',id,'--title',f'최근검증 {index:02}','--markdown',PREFIX)
            cli('close',id)
        menu=open_menu();REPORT['menu30']=menu
        assert len(entries(menu))==30 and not any(row['title'].startswith('더보기(') for row in menu)
        choose('최근검증 기준')
        id=PREFIX+'-overflow';created.append(id)
        cli('open',id,'--title','최근검증 초과','--markdown',PREFIX);cli('close',id)
        menu=open_menu();REPORT['menu31']=menu
        rows=entries(menu);assert len(rows)==31 and rows[-1]['title']=='더보기(1)'
        assert all(row['enabled'] for row in rows)
        assert not any(PREFIX in row['title'] or row['title'].startswith(('[터미널]','[에이전트]')) for row in rows)
        snapshot('menu31')
        choose('최근검증 기준')
        # Update only the body of an existing, older note: next open must reorder.
        cli('update',proof,'--markdown','# '+PREFIX+'\n본문만 갱신')
        menu=open_menu();assert entries(menu)[0]['title']=='최근검증 기준';REPORT['menu_after_body_update']=menu
        choose('더보기(1)')
        check('30 has no More; 31 has exactly 30 entries plus More(1); body-only update reorders on next open')
        wait(lambda:any('메모 관리' in w.get('kCGWindowName','') for w in native()['windows']))
        REPORT['native_manager']=native()
        check('More opens the native manager belonging to the same PID')
        REPORT['result']='passed'
    finally:
        for id in created: cli('close',id)
        run(temp/'pointer','move',original['x'],original['y'])
        if front!=str(pid):apple(f'tell application "System Events" to set frontmost of (first application process whose unix id is {front}) to true')
        REPORT['created']=created
        (OUT/'native-report.json').write_text(json.dumps(REPORT,ensure_ascii=False,indent=2))
