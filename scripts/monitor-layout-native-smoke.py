#!/usr/bin/env python3
"""Explicitly starts/stops only this worktree's dev app and edits only test-note fixtures.
Run after dev:build with GUI/instance coordination. Refuses unrelated data/processes.
Uses the saved journal to simulate recovery on startup, never real display changes.
"""
import json, subprocess, pathlib, time, os, signal
ROOT=pathlib.Path(__file__).resolve().parents[1]
CLI=ROOT/'scripts/agent-gibbon-dev'
INFO=json.loads(subprocess.check_output(['node','scripts/dev.mjs','info'],cwd=ROOT,text=True))
BINARY=INFO['binary'];ID='monitor20-20261115-check';LABEL='note-'+ID
BODY='MONITOR20-NATIVE-20261115 검증 전용 메모';COLOR='#a1b2c3'
STORE=pathlib.Path.home()/'Library/Application Support/org.egoing.floatnote/notes'/INFO['id']/'docs'
JOURNAL=STORE/'.session-windows.json'
OUT=ROOT/'docs/verification/monitor-layout/native-results.json'
result={'root':str(ROOT),'instance':INFO['id'],'binary':BINARY,'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'test_id':ID,'test_body':BODY,'physical_monitor_changes':False,'phases':[]}
started=[];wrappers=[]
def pids():
    rows=subprocess.check_output(['ps','-axo','pid=,comm='],text=True).splitlines()
    return [int(row.strip().split(None,1)[0]) for row in rows if row.strip().split(None,1)[-1]==BINARY]
def call(*args,ok=True):
    r=subprocess.run([str(CLI),*args],cwd=ROOT,text=True,capture_output=True,timeout=25)
    data=json.loads(r.stdout.strip().splitlines()[-1])
    if ok:assert r.returncode==0 and data['ok'],data
    return data
def wait(fn,seconds=18):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        value=fn()
        if value:return value
        time.sleep(.2)
    raise AssertionError('condition timed out')
def read():
    try:return json.loads(JOURNAL.read_text())
    except (FileNotFoundError,json.JSONDecodeError):return None
def entry(s):return next(w for w in s['windows'] if w['window']['label']==LABEL)
def inspected():
    d=call('inspect','--id',ID)
    assert d['instance']==INFO['id'] and d['id']==ID and d['note']['markdown']==BODY and d['note']['color']==COLOR,d
    return d
def summary(d):return {k:d[k] for k in ['instance','id','position','size','visible','focused','folded','always_on_top','capture_toolbar_below']}
def wait_saved(predicate):
    return wait(lambda: (s if (s:=read()) and predicate(s) else None))
def stop(pid):
    assert pid in pids() and pid in started
    os.kill(pid,signal.SIGTERM);wait(lambda:pid not in pids())
def serve():
    assert not pids()
    wrappers.append(subprocess.Popen([str(CLI),'--serve'],cwd=ROOT,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL))
    pid=wait(lambda:next(iter(pids()),None));started.append(pid)
    wait(lambda:call('diagnostics',ok=False).get('ok'))
    return pid
def write(s):
    temp=JOURNAL.with_suffix('.smoke-temp');temp.write_text(json.dumps(s));temp.replace(JOURNAL)
def inside(r,a):return r['x']>=a['x'] and r['y']>=a['y'] and r['x']+r['width']<=a['x']+a['width'] and r['y']+r['height']<=a['y']+a['height']
try:
    assert not pids(),'existing instance must be left untouched'
    assert not STORE.exists(), 'fresh dedicated instance required; existing data must be preserved'
    before=call('list',ok=False);result['before']=before
    call('open','--id',ID,'--markdown',BODY,'--color',COLOR)
    pid=wait(lambda:next(iter(pids()),None));started.append(pid)
    d=inspected();assert d['visible'];result['phases'].append({'phase':'new-note','pid':pid,'inspect':summary(d)})
    call('close','--id',ID)
    s=wait_saved(lambda s: LABEL in s.get('monitor_layout',{}).get('records',{}) and not entry(s)['window']['visible'])
    d=inspected();assert not d['visible'] and not d['focused']
    record=s['monitor_layout']['records'][LABEL];original=json.loads(json.dumps(record['last']));original_screen=json.loads(json.dumps(record['screen']))
    result['phases'].append({'phase':'checkpoint-hidden','inspect':summary(d),'screen_count_in_records':len({str(r['screen']['bounds']) for r in s['monitor_layout']['records'].values()})})
    stop(pid)
    # Simulate a disconnected source display in THIS test note's saved data.
    # This is a startup-recovery fixture, not a physical display transition.
    s=read();record=s['monitor_layout']['records'][LABEL]
    fake=json.loads(json.dumps(original_screen));fake['name']='monitor20-disconnected-fixture'
    shift=50000;fake['bounds']['x']+=shift;fake['work']['x']+=shift
    home=json.loads(json.dumps(original));home['x']+=shift
    record.update(last=home,screen=fake,home=None)
    unit=fake['scale'];entry(s)['window'].update(x=round(home['x']*unit),y=round(home['y']*unit),width=round(home['width']*unit),height=round(home['height']*unit))
    write(s);pid=serve()
    s=wait_saved(lambda s: s['monitor_layout']['records'][LABEL]['home'] is not None)
    record=s['monitor_layout']['records'][LABEL]
    assert record['home']['frame']==home and inside(record['last'],record['screen']['work']),record
    d=inspected();assert not d['visible'] and not d['focused']
    result['phases'].append({'phase':'seeded-disconnected-startup','pid':pid,'inspect':summary(d),'original_frame':record['home']['frame'],'temporary_frame':record['last'],'work_area':record['screen']['work'],'diagnostics':call('diagnostics')['monitor_layout']})
    stop(pid)
    # A second restart while still absent must keep the original home intact.
    pid=serve();s=wait_saved(lambda s: s['monitor_layout']['records'][LABEL]['home'] is not None)
    assert s['monitor_layout']['records'][LABEL]['home']['frame']==home
    d=inspected();assert not d['visible'];result['phases'].append({'phase':'temporary-home-survives-restart','pid':pid,'inspect':summary(d)})
    stop(pid)
    # Match the real original screen in a seeded journal to exercise repatriation.
    s=read();s['monitor_layout']['records'][LABEL]['home']={'frame':original,'screen':original_screen};write(s)
    pid=serve();s=wait_saved(lambda s: s['monitor_layout']['records'][LABEL]['home'] is None)
    r=s['monitor_layout']['records'][LABEL];assert r['last']==original,r
    d=inspected();assert not d['visible'] and not d['focused'];result['phases'].append({'phase':'seeded-matching-screen-startup-return','pid':pid,'inspect':summary(d),'returned_frame':r['last']})
    result['ok']=True
finally:
    active=pids()
    for pid in active:
        if pid in started:
            try:call('close','--id',ID)
            finally:stop(pid)
    for w in wrappers:
        try:w.wait(timeout=5)
        except subprocess.TimeoutExpired:pass
    result['started_pids']=started;result['remaining_instance_pids']=pids()
    OUT.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'ok':result.get('ok',False),'phases':[p['phase'] for p in result['phases']],'evidence':str(OUT)},ensure_ascii=False))
