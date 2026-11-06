"""전용 UI 창 준비/정리. GUI 입력은 공식 Orca 도구로 수행한다."""
import runpy,pathlib,json,subprocess,tempfile,time,sys,os
d=runpy.run_path(str(pathlib.Path(__file__).with_name('run-product.py')),run_name='helpers')
globals().update({k:v for k,v in d.items() if not k.startswith('__')})
root=HERE.parents[2];context=HERE/'ui-session.json'
api('SetProcessDpiAwarenessContext',-4)
if sys.argv[1]=='setup':
    foreground=u.GetForegroundWindow();cursor=POINT();u.GetCursorPos(c.byref(cursor))
    temp=pathlib.Path(tempfile.mkdtemp(prefix='gibbon-product-ui-6b023e9f-'));binary=temp/'Issue86ProductUI.exe'
    result=cmd('compile',[r'C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe','/nologo','/target:winexe','/r:System.Windows.Forms.dll','/r:System.Drawing.dll','/out:'+str(binary),str(HERE/'TestWindow.cs')]);assert result.returncode==0
    owned=[]
    for role in ('UIA','UIB'):
        delay=sys.argv[2] if len(sys.argv)>2 and role=='UIA' else '0'
        proc=subprocess.Popen([str(binary),str(OUT),role,'0',delay])
        for _ in range(100):
            if (OUT/(role+'-ready.json')).exists():break
            time.sleep(.1)
        ident=json.loads((OUT/(role+'-ready.json')).read_text(encoding='utf-8-sig'));assert ident['pid']==proc.pid
        owned.append(ident);log('ui-owned',role=role,state=snap(ident['hwnd']))
    shortcut=pathlib.Path(os.environ['APPDATA'])/'Microsoft/Windows/Start Menu/Programs/ISSUE86-PRODUCT-6b023e9f-UI.lnk';assert not shortcut.exists()
    env={**os.environ,'GIBBON_TEST_LINK':str(shortcut),'GIBBON_TEST_EXE':str(binary),'GIBBON_TEST_ARGS':'"'+str(OUT)+'" UILaunch','GIBBON_TEST_CWD':str(temp)}
    subprocess.run(['powershell.exe','-NoProfile','-Command',r'$s=(New-Object -ComObject WScript.Shell).CreateShortcut($env:GIBBON_TEST_LINK);$s.TargetPath=$env:GIBBON_TEST_EXE;$s.Arguments=$env:GIBBON_TEST_ARGS;$s.WorkingDirectory=$env:GIBBON_TEST_CWD;$s.Save()'],env=env,check=True)
    api('SetWindowPos',owned[1]['hwnd'],0,20,20,1500,930,0x10)
    activate(owned[1]['hwnd'])
    context.write_text(json.dumps(dict(out=str(OUT),binary=str(binary),owned=owned,foreground=foreground,cursor=[cursor.x,cursor.y],shortcut=str(shortcut)),indent=2),encoding='utf-8')
    print(context.read_text(),flush=True)
elif sys.argv[1]=='cleanup':
    state=json.loads(context.read_text())
    for ident in state['owned']:
        try:
            observed=snap(ident['hwnd']);assert observed['pid']==ident['pid'] and observed['exe']==state['binary']
            api('PostMessageW',ident['hwnd'],0x10,0,0)
        except BaseException:pass
    path=pathlib.Path(state['shortcut'])
    if path.is_file():path.unlink()
    bind(u,'IsWindow',w.BOOL,[w.HWND])
    for _ in range(30):
        if not any(u.IsWindow(x['hwnd']) for x in state['owned']):break
        time.sleep(.1)
    activate(state['foreground']);api('SetCursorPos',*state['cursor'])
    current=POINT();u.GetCursorPos(c.byref(current))
    log('final-user-state',foreground_expected=state['foreground'],foreground_actual=u.GetForegroundWindow(),cursor_expected=state['cursor'],cursor_actual=[current.x,current.y],owned_windows_closed=not any(u.IsWindow(x['hwnd']) for x in state['owned']))
