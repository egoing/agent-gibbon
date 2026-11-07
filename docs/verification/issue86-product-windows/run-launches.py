"""제품 CLI 실행·취소·시간 초과 실증. 생성한 바로가기와 전용 앱만 정리한다."""
import runpy, pathlib, subprocess, json, time, tempfile, os, traceback
d=runpy.run_path(str(pathlib.Path(__file__).with_name('run-product.py')),run_name='helpers')
globals().update({k:v for k,v in d.items() if not k.startswith('__')})
root=HERE.parents[2];cli=str(root/'scripts/agent-gibbon-dev.cmd');checks={};owned=[];shortcuts=[]
foreground=u.GetForegroundWindow();cursor=POINT();u.GetCursorPos(c.byref(cursor))
def product(name,*args):
    p=subprocess.run([cli,*args],cwd=root,capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=40)
    value=json.loads(p.stdout)
    if 'windows' in value:value['windows']=[x for x in value['windows'] if 'ISSUE86-PRODUCT-' in x.get('title','')]
    if 'apps' in value:value['apps']=[x for x in value['apps'] if 'ISSUE86-PRODUCT-' in x.get('label','')]
    log('product',name=name,args=args,exit=p.returncode,response=value);return value
def setup(role,delay=0):
    temp=pathlib.Path(tempfile.mkdtemp(prefix='gibbon-product-launch-6b023e9f-'));binary=temp/(role+'.exe')
    build=cmd('compile-'+role,[r'C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe','/nologo','/target:winexe','/r:System.Windows.Forms.dll','/r:System.Drawing.dll','/out:'+str(binary),str(HERE/'TestWindow.cs')]);assert build.returncode==0
    shortcut=pathlib.Path(os.environ['APPDATA'])/'Microsoft/Windows/Start Menu/Programs'/('ISSUE86-PRODUCT-6b023e9f-'+role+'.lnk')
    assert not shortcut.exists();shortcuts.append(shortcut)
    script=r'''$s=(New-Object -ComObject WScript.Shell).CreateShortcut($env:GIBBON_TEST_LINK); $s.TargetPath=$env:GIBBON_TEST_EXE; $s.Arguments=$env:GIBBON_TEST_ARGS; $s.WorkingDirectory=$env:GIBBON_TEST_CWD; $s.Save()'''
    args='"'+str(OUT)+'" '+role+' '+str(delay)
    p=subprocess.run(['powershell.exe','-NoProfile','-Command',script],env={**os.environ,'GIBBON_TEST_LINK':str(shortcut),'GIBBON_TEST_EXE':str(binary),'GIBBON_TEST_ARGS':args,'GIBBON_TEST_CWD':str(temp)},capture_output=True);assert p.returncode==0
    owned.append((role,binary));return shortcut,binary
def wait_ready(role,seconds=20):
    for _ in range(int(seconds*10)):
        path=OUT/(role+'-ready.json')
        if path.exists():return json.loads(path.read_text(encoding='utf-8-sig'))
        time.sleep(.1)
    raise RuntimeError('Own test app readiness timeout: '+role)
try:
    info=json.loads(subprocess.check_output(['node','scripts/dev.mjs','info'],cwd=root,text=True));log('provenance',instance=info,sha=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip())
    shortcut,binary=setup('Launch')
    catalog=product('catalog','app','list');assert any(x['id']==str(shortcut) for x in catalog['apps'])
    result=product('launch','app','open','--app-id',str(shortcut),'--region','220,180,640,420');assert result['ok'];launch=result['launch']
    ident=wait_ready('Launch'); state=snap(ident['hwnd']);assert state['pid']==ident['pid'] and pathlib.Path(state['exe'])==binary
    for _ in range(100):
        result=product('poll-launch','app','sessions');status=result['launches'][launch]
        if status['state']!='pending':break
        time.sleep(.2)
    checks['new_shortcut_launch']=status['state']=='attached' and snap(ident['hwnd'])['topmost']
    events=(OUT/'Launch-events.tsv').read_text(encoding='utf-8-sig')
    checks['shortcut_working_directory']=('\tcwd\t'+str(binary.parent)) in events
    if status['state']=='attached':product('release-launch','app','release','--session',status['result']['session'])
    before=snap(ident['hwnd'])
    result=product('existing-disambiguation','app','open','--app-id',str(shortcut),'--region','300,250,640,420')
    checks['existing_explicit_selection']=len(result.get('existing_windows',[]))==1 and snap(ident['hwnd'])['rect']==before['rect'] and not snap(ident['hwnd'])['topmost']
    delayed,delayed_exe=setup('Cancel',3500);product('catalog-cancel','app','list')
    result=product('launch-cancel','app','open','--app-id',str(delayed),'--region','220,180,640,420');launch=result['launch']
    product('cancel','app','cancel','--session',launch)
    # Cancellation may win before process creation; either no process or a normal untouched app is valid.
    time.sleep(5)
    ready=OUT/'Cancel-ready.json';state=snap(json.loads(ready.read_text())['hwnd']) if ready.exists() else None
    result=product('after-cancel','app','sessions')
    checks['cancel_no_late_attach']=result['launches'][launch]['state']=='cancelled' and (state is None or not state['topmost']) and not result['sessions']
    log('cancel-observed',state=state)
    delayed,delayed_exe=setup('Timeout',14500);product('catalog-timeout','app','list')
    result=product('launch-timeout','app','open','--app-id',str(delayed),'--region','220,180,640,420');launch=result['launch']
    time.sleep(13)
    result=product('after-timeout','app','sessions');status=result['launches'][launch]
    ident=wait_ready('Timeout',6);state=snap(ident['hwnd'])
    checks['pending_timeout']=status['state']=='failed' and not state['topmost'] and not result['sessions']
    log('timeout-observed',status=status,state=state)
except BaseException:log('failure',traceback=traceback.format_exc())
finally:
    for role,binary in owned:
        path=OUT/(role+'-ready.json')
        if path.exists():
            try:
                ident=json.loads(path.read_text(encoding='utf-8-sig'));state=snap(ident['hwnd'])
                assert state['pid']==ident['pid'] and pathlib.Path(state['exe'])==binary
                api('PostMessageW',ident['hwnd'],0x10,0,0)
            except BaseException:log('cleanup-error',role=role,traceback=traceback.format_exc())
    for path in shortcuts:
        if path.is_file():path.unlink();log('shortcut-removed',path=str(path))
    if foreground:activate(foreground)
    api('SetCursorPos',cursor.x,cursor.y)
    (OUT/'summary.json').write_text(json.dumps(checks,indent=2),encoding='utf-8');print('EVIDENCE='+str(OUT),flush=True)
sys.exit(0 if len(checks)==5 and all(checks.values()) else 1)
