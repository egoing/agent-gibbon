"""자기 개발 앱의 비정상/정상 종료와 로컬 복원 기록을 실증한다."""
import runpy,pathlib,json,subprocess,time,os,traceback
d=runpy.run_path(str(pathlib.Path(__file__).with_name('run-product.py')),run_name='helpers')
globals().update({k:v for k,v in d.items() if not k.startswith('__')})
api('SetProcessDpiAwarenessContext',-4)
root=HERE.parents[2];cli=str(root/'scripts/agent-gibbon-dev.cmd');ctx=json.loads((HERE/'ui-session.json').read_text());a=ctx['owned'][0]['hwnd'];checks={}
info=json.loads(subprocess.check_output(['node','scripts/dev.mjs','info'],cwd=root,text=True))
def product(name,*args):
    p=subprocess.run([cli,*args],cwd=root,capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=40);value=json.loads(p.stdout)
    if 'windows' in value:value['windows']=[x for x in value['windows'] if 'ISSUE86-PRODUCT-' in x.get('title','')]
    log('product',name=name,args=args,exit=p.returncode,response=value);return value
def server():
    script='Get-CimInstance Win32_Process -Filter "Name = \'agent-gibbon.exe\'" | Where-Object { $_.ExecutablePath -eq $env:GIBBON_TEST_BINARY } | Select-Object ProcessId,ExecutablePath | ConvertTo-Json -Compress'
    value=json.loads(subprocess.check_output(['powershell.exe','-NoProfile','-Command',script],env={**os.environ,'GIBBON_TEST_BINARY':info['binary']},text=True))
    assert isinstance(value,dict) and value['ExecutablePath'].lower()==info['binary'].lower();log('server-owned',process=value);return value['ProcessId']
def crash():
    pid=server();subprocess.run(['powershell.exe','-NoProfile','-Command','Stop-Process -Id '+str(pid)],check=True);time.sleep(.8)
def restart():
    assert product('restart','open','--id','issue86-product-6b023e9f','--markdown','ISSUE86_PRODUCT_6b023e9f')['ok']
    note=product('restart-inspect','inspect','--id','issue86-product-6b023e9f');assert note['instance']==info['id'] and note['note']['markdown']=='ISSUE86_PRODUCT_6b023e9f'
    product('hide-note','close','--id','issue86-product-6b023e9f');server();return note
try:
    before=snap(a);assert pathlib.Path(before['exe'])==pathlib.Path(ctx['binary'])
    note=product('provenance-inspect','inspect','--id','issue86-product-6b023e9f');assert note['instance']==info['id']
    journal_root=pathlib.Path(note['storage_path']).parents[1]/'.external-recovery'
    product('inventory','app','windows');attached=product('attach-recovery','app','attach','--window',str(a),'--region','220,180,640,420');assert attached['ok']
    journals=list(journal_root.glob('*.json'));assert len(journals)==1
    saved=json.loads(journals[0].read_text());log('journal-before-crash',path=str(journals[0]),journal=saved)
    crash();assert snap(a)['topmost'];restart();after=snap(a)
    checks['crash_journal_restore']=all(before[k]==after[k] for k in ('rect','topmost','visible','placement')) and not list(journal_root.glob('*.json'))
    log('crash-recovery-observed',before=before,after=after)
    # Simulate an invalid/replaced window claim on an OWN window only. This is not empirical HWND reuse.
    product('inventory-guard','app','windows');attached=product('attach-guard','app','attach','--window',str(a),'--region','300,240,640,420');assert attached['ok']
    path=next(journal_root.glob('*.json'));saved=json.loads(path.read_text());crash()
    bind(u,'RemovePropW',w.HANDLE,[w.HWND,w.LPCWSTR]);removed=u.RemovePropW(a,saved['claim']);assert removed==1
    changed=snap(a);restart();after=snap(a)
    checks['guard_mismatch_no_touch']=all(changed[k]==after[k] for k in ('rect','topmost','visible','placement')) and not list(journal_root.glob('*.json'))
    log('simulated-guard-mismatch-observed',removed_property=saved['claim'],before=changed,after=after)
    # Restore only our fixture after the deliberately invalidated claim, then reattach for orderly WM_QUIT.
    restore(a,before)
    product('inventory-orderly','app','windows');attached=product('attach-orderly','app','attach','--window',str(a),'--region','220,180,640,420');assert attached['ok']
    pid=server();windows=[];cbtype=c.WINFUNCTYPE(w.BOOL,w.HWND,w.LPARAM)
    def cb(h,param):
        p=w.DWORD();tid=u.GetWindowThreadProcessId(h,c.byref(p))
        if p.value==pid:windows.append((h,tid))
        return True
    callback=cbtype(cb);u.EnumWindows(callback,0);assert windows
    bind(u,'PostThreadMessageW',w.BOOL,[w.DWORD,w.UINT,w.WPARAM,w.LPARAM])
    api('PostThreadMessageW',windows[0][1],0x12,0,0)
    for _ in range(50):
        time.sleep(.1)
        if not snap(a)['topmost']:break
    after=snap(a);checks['orderly_wm_quit_restore']=all(before[k]==after[k] for k in ('rect','topmost','visible','placement')) and not list(journal_root.glob('*.json'))
    log('orderly-shutdown-observed',trigger='Win32 PostThreadMessage WM_QUIT to verified own GUI thread; not tray UI',before=before,after=after)
    restart()
except BaseException:log('failure',traceback=traceback.format_exc())
finally:
    (OUT/'summary.json').write_text(json.dumps(checks,indent=2),encoding='utf-8');print('EVIDENCE='+str(OUT),flush=True)
sys.exit(0 if len(checks)==3 and all(checks.values()) else 1)
