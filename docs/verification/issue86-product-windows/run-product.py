"""#86 전용 프로세스 두 개만 제어하는 Windows 선행 실증. 제품 코드 아님.
Usage: python docs/verification/issue86-windows-poc/run-poc.py
증거는 새 run-* 폴더, 실행 파일은 로컬 임시 폴더에 생성한다.
"""
import ctypes as c
from ctypes import wintypes as w
import datetime, json, os, pathlib, platform, subprocess, sys, tempfile, time, traceback
from PIL import ImageGrab
sys.stdout.reconfigure(encoding='utf-8')

HERE = pathlib.Path(__file__).resolve().parent
OUT = HERE / ("run-" + datetime.datetime.now().strftime("%Y%m%d-%H%M%S"))
OUT.mkdir()
u = c.WinDLL("user32", use_last_error=True)
k = c.WinDLL("kernel32", use_last_error=True)
class RECT(c.Structure): _fields_ = [(x,w.LONG) for x in ("left","top","right","bottom")]
class POINT(c.Structure): _fields_ = [("x",w.LONG),("y",w.LONG)]
class WP(c.Structure): _fields_ = [("length",w.UINT),("flags",w.UINT),("showCmd",w.UINT),("min",POINT),("max",POINT),("normal",RECT)]
class MI(c.Structure): _fields_ = [("dx",w.LONG),("dy",w.LONG),("mouseData",w.DWORD),("dwFlags",w.DWORD),("time",w.DWORD),("extra",c.c_size_t)]
class KI(c.Structure): _fields_ = [("vk",w.WORD),("scan",w.WORD),("flags",w.DWORD),("time",w.DWORD),("extra",c.c_size_t)]
class UNION(c.Union): _fields_ = [("mi",MI),("ki",KI)]
class INPUT(c.Structure): _anonymous_=("value",); _fields_=[("type",w.DWORD),("value",UNION)]

def bind(lib,name,ret,args):
    f=getattr(lib,name); f.restype=ret; f.argtypes=args; return f
bind(u,"GetForegroundWindow",w.HWND,[])
bind(u,"GetWindowTextW",c.c_int,[w.HWND,w.LPWSTR,c.c_int])
bind(u,"GetWindowRect",w.BOOL,[w.HWND,c.POINTER(RECT)])
bind(u,"GetWindowPlacement",w.BOOL,[w.HWND,c.POINTER(WP)])
bind(u,"SetWindowPlacement",w.BOOL,[w.HWND,c.POINTER(WP)])
bind(u,"GetWindowLongW",w.LONG,[w.HWND,c.c_int])
bind(u,"GetWindowThreadProcessId",w.DWORD,[w.HWND,c.POINTER(w.DWORD)])
bind(u,"SetWindowPos",w.BOOL,[w.HWND,w.HWND,c.c_int,c.c_int,c.c_int,c.c_int,w.UINT])
bind(u,"SetForegroundWindow",w.BOOL,[w.HWND])
bind(u,"GetDpiForWindow",w.UINT,[w.HWND])
bind(u,"GetWindow",w.HWND,[w.HWND,w.UINT])
bind(u,"WindowFromPoint",w.HWND,[POINT])
bind(u,"GetAncestor",w.HWND,[w.HWND,w.UINT])
bind(u,"GetCursorPos",w.BOOL,[c.POINTER(POINT)])
bind(u,"SetCursorPos",w.BOOL,[c.c_int,c.c_int])
bind(u,"IsWindowVisible",w.BOOL,[w.HWND])
bind(u,"SendInput",w.UINT,[w.UINT,c.POINTER(INPUT),c.c_int])
bind(u,"SendMessageW",c.c_ssize_t,[w.HWND,w.UINT,w.WPARAM,w.LPARAM])
bind(u,"PostMessageW",w.BOOL,[w.HWND,w.UINT,w.WPARAM,w.LPARAM])
bind(u,"SetProcessDpiAwarenessContext",w.BOOL,[c.c_void_p])
bind(u,"AttachThreadInput",w.BOOL,[w.DWORD,w.DWORD,w.BOOL])
bind(k,"GetCurrentThreadId",w.DWORD,[])
bind(k,"OpenProcess",w.HANDLE,[w.DWORD,w.BOOL,w.DWORD])
bind(k,"QueryFullProcessImageNameW",w.BOOL,[w.HANDLE,w.DWORD,w.LPWSTR,c.POINTER(w.DWORD)])
bind(k,"CloseHandle",w.BOOL,[w.HANDLE])

def log(kind, **data):
    row=dict(utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),kind=kind,**data)
    with (OUT/"raw.jsonl").open("a",encoding="utf-8") as f: f.write(json.dumps(row,ensure_ascii=False)+"\n")
    print(json.dumps(row,ensure_ascii=False),flush=True)

def api(name,*args):
    c.set_last_error(0); value=getattr(u,name)(*args)
    log("api",name=name,result=int(value or 0),last_error=c.get_last_error(),args=[str(a) for a in args])
    return value

def rlist(r): return [r.left,r.top,r.right,r.bottom]
def snap(h):
    r=RECT(); p=WP(); p.length=c.sizeof(p)
    assert u.GetWindowRect(h,c.byref(r)) and u.GetWindowPlacement(h,c.byref(p))
    pid=w.DWORD(); u.GetWindowThreadProcessId(h,c.byref(pid))
    ph=k.OpenProcess(0x1000,False,pid.value); buf=c.create_unicode_buffer(32768); size=w.DWORD(len(buf))
    try: assert k.QueryFullProcessImageNameW(ph,0,buf,c.byref(size))
    finally: k.CloseHandle(ph)
    return dict(hwnd=h,hwnd_hex=hex(h),pid=pid.value,exe=buf.value,rect=rlist(r),
                topmost=bool(u.GetWindowLongW(h,-20)&8),visible=bool(u.IsWindowVisible(h)),dpi=u.GetDpiForWindow(h),
                placement=dict(flags=p.flags,showCmd=p.showCmd,min=[p.min.x,p.min.y],max=[p.max.x,p.max.y],normal=rlist(p.normal)),
                placement_hex=bytes(p).hex())

def capture(name,rect):
    time.sleep(.6)
    image=ImageGrab.grab(bbox=tuple(rect),all_screens=True)
    path=OUT/(name+".png"); image.save(path)
    log("screenshot",path=path.name,screen_bbox=rect,size=list(image.size))

def cmd(name,args):
    p=subprocess.run(args,capture_output=True,text=True,encoding="utf-8",errors="replace",timeout=45)
    (OUT/(name+".stdout.txt")).write_text(p.stdout,encoding="utf-8")
    (OUT/(name+".stderr.txt")).write_text(p.stderr,encoding="utf-8")
    log("command",name=name,args=args,exit=p.returncode)
    return p

def click(x,y,expected_root):
    root=u.GetAncestor(u.WindowFromPoint(POINT(x,y)),2)
    log("input-target",point=[x,y],observed_root=root,expected_root=expected_root)
    assert root==expected_root, "Input point is covered by a different window; no input sent"
    assert api("SetCursorPos",x,y)
    arr=(INPUT*2)(); arr[0].type=0; arr[0].mi.dwFlags=2; arr[1].type=0; arr[1].mi.dwFlags=4
    assert api("SendInput",2,arr,c.sizeof(INPUT))==2
    time.sleep(.25)

def point_in(h,dx,dy):
    r=RECT(); assert u.GetWindowRect(h,c.byref(r)); return r.left+dx,r.top+dy

def activate(h):
    # Temporary input-queue attachment, only while requesting activation of an owned HWND.
    tid=u.GetWindowThreadProcessId(u.GetForegroundWindow(),None)
    current=k.GetCurrentThreadId()
    attached=api("AttachThreadInput",current,tid,True) if tid!=current else False
    try: api("SetForegroundWindow",h)
    finally:
        if attached: api("AttachThreadInput",current,tid,False)
    time.sleep(.4)
    log("activation-observation",requested=h,foreground=u.GetForegroundWindow())

def desktop_evidence(a,b):
    dwm=c.WinDLL("dwmapi")
    bind(dwm,"DwmGetWindowAttribute",c.c_long,[w.HWND,w.DWORD,c.c_void_p,w.DWORD])
    for label,h in (("A",a),("B",b),("original-foreground",foreground)):
        cloak=w.DWORD(); hr=dwm.DwmGetWindowAttribute(h,14,c.byref(cloak),c.sizeof(cloak))
        log("desktop-observation",label=label,state=snap(h),dwm_cloaked=cloak.value,dwm_hresult=hr)
    order=[]; h=u.GetWindow(a,0)
    for i in range(500):
        if not h: break
        if h in (a,b,foreground): order.append(dict(index=i,hwnd=h))
        h=u.GetWindow(h,2)
    log("zorder-observation",own_and_original_foreground_only=order)

def keytext(text,h):
    assert u.GetForegroundWindow()==h,"Refusing keyboard input into another app"
    assert not any(u.GetAsyncKeyState(v)&0x8000 for v in (0x10,0x11,0x12,0x5b,0x5c)),"User modifier held"
    arr=(INPUT*(2*len(text)))()
    for i,ch in enumerate(text):
        for j in (0,1): arr[2*i+j].type=1; arr[2*i+j].ki.scan=ord(ch); arr[2*i+j].ki.flags=4|(2 if j else 0)
    assert api("SendInput",len(arr),arr,c.sizeof(INPUT))==len(arr)
    time.sleep(.6)

def restore(h,initial):
    p=WP.from_buffer_copy(bytes.fromhex(initial["placement_hex"]))
    api("SetWindowPos",h,-1 if initial["topmost"] else -2,0,0,0,0,0x13)
    assert api("SetWindowPlacement",h,c.byref(p))
    time.sleep(.4)
    actual=snap(h)
    log("restore-observation",initial=initial,actual=actual,exact=all(actual[x]==initial[x] for x in ("rect","placement","topmost","visible")))
    return actual

if __name__ == '__main__':
    root=HERE.parents[2]
    cli=str(root/'scripts/agent-gibbon-dev.cmd')
    def product(name,*args,private=False):
        p=subprocess.run([cli,*args],capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=40,cwd=root)
        data=json.loads(p.stdout)
        # Catalog and window inventory may contain unrelated user metadata. Persist only owned entries.
        if private:
            data={**data,'windows':[r for r in data.get('windows',[]) if 'ISSUE86-PRODUCT-' in r.get('title','')], 'apps':[r for r in data.get('apps',[]) if 'ISSUE86-PRODUCT-' in r.get('label','')]}
        log('product',name=name,args=args,exit=p.returncode,response=data)
        return data
    api('SetProcessDpiAwarenessContext',-4)
    foreground=u.GetForegroundWindow(); cursor=POINT();u.GetCursorPos(c.byref(cursor))
    procs=[];owned={};initial={};checks={};session=None
    try:
        info=json.loads(subprocess.check_output(['node','scripts/dev.mjs','info'],cwd=root,text=True))
        log('provenance',root=str(root),source_sha=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),instance=info,os=platform.platform())
        note='issue86-product-6b023e9f'
        assert product('open-note','open','--id',note,'--markdown','ISSUE86_PRODUCT_6b023e9f')['ok']
        inspected=product('inspect-note','inspect','--id',note)
        assert inspected['ok'] and inspected['instance']==info['id'] and inspected['note']['markdown']=='ISSUE86_PRODUCT_6b023e9f'
        proc_script='Get-CimInstance Win32_Process -Filter "Name = \'agent-gibbon.exe\'" | Where-Object { $_.ExecutablePath -eq $env:GIBBON_TEST_BINARY } | Select-Object ProcessId,ExecutablePath | ConvertTo-Json -Compress'
        servers=json.loads(subprocess.check_output(['powershell.exe','-NoProfile','-Command',proc_script],env={**os.environ,'GIBBON_TEST_BINARY':info['binary']},text=True))
        if isinstance(servers,dict):servers=[servers]
        assert len(servers)==1
        server_pid=servers[0]['ProcessId'];titles=[]
        callback_type=c.WINFUNCTYPE(w.BOOL,w.HWND,w.LPARAM)
        def collect_window(h,param):
            window_pid=w.DWORD();u.GetWindowThreadProcessId(h,c.byref(window_pid))
            if window_pid.value==server_pid:
                title=c.create_unicode_buffer(2048);u.GetWindowTextW(h,title,len(title))
                titles.append({'hwnd':h,'pid':server_pid,'title':title.value})
            return True
        collect_callback=callback_type(collect_window);u.EnumWindows(collect_callback,0)
        assert any(note in x['title'] for x in titles)
        log('server-provenance',server=servers,windows=titles)
        product('hide-note','close','--id',note)
        temp=pathlib.Path(tempfile.mkdtemp(prefix='gibbon-product-6b023e9f-'));binary=temp/'Issue86ProductWindow.exe'
        p=cmd('compile',[r'C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe','/nologo','/target:winexe','/r:System.Windows.Forms.dll','/r:System.Drawing.dll','/out:'+str(binary),str(HERE/'TestWindow.cs')]);assert p.returncode==0
        for role in ('A','B'):
            p=subprocess.Popen([str(binary),str(OUT),role]);procs.append(p)
            for _ in range(100):
                if (OUT/(role+'-ready.json')).exists():break
                time.sleep(.1)
            ident=json.loads((OUT/(role+'-ready.json')).read_text(encoding='utf-8-sig'));owned[role]=ident
            state=snap(ident['hwnd']); assert state['pid']==p.pid and pathlib.Path(state['exe'])==binary
            initial[role]=state;log('owned-initial',role=role,state=state)
        a=owned['A']['hwnd'];b=owned['B']['hwnd']
        product('inventory','app','windows',private=True)
        result=product('attach','app','attach','--window',str(a),'--region','220,180,640,420')
        assert result['ok'];session=result['session'];time.sleep(.5)
        observed=snap(a);log('position-observed',state=observed)
        checks['move_resize']=observed['rect']==[220,180,860,600] and observed['topmost']
        selector=['--app','pid:'+str(owned['A']['pid']),'--window-index','0']
        cmd('orca-attached',['orca','computer','get-app-state',*selector,'--restore-window','--json'])
        capture('01-attached',observed['rect'])
        cmd('orca-B',['orca','computer','get-app-state','--app','pid:'+str(owned['B']['pid']),'--window-index','0','--restore-window','--no-screenshot','--json'])
        activate(b)
        api('SetWindowPos',b,0,0,0,0,0,0x13)
        title_point=POINT(*point_in(b,80,15))
        title_root=u.GetAncestor(u.WindowFromPoint(title_point),2)
        log('B-title-hit-before-input',root=title_root,expected=b,point=[title_point.x,title_point.y])
        assert title_root==b,'Refuse input until owned ordinary B title is exposed'
        cmd('orca-B-actual-click',['orca','computer','click','--app','pid:'+str(owned['B']['pid']),'--window-index','0','--x','80','--y','15','--json','--no-screenshot'])
        overlap=POINT(300,300); hit=u.GetAncestor(u.WindowFromPoint(overlap),2)
        checks['topmost_after_B']=u.GetForegroundWindow()==b and hit==a and snap(a)['topmost'] and not snap(b)['topmost']
        log('topmost-observed',foreground=u.GetForegroundWindow(),expected_B=b,overlap_root=hit,expected_A=a)
        capture('02-topmost',snap(a)['rect'])
        cmd('orca-focus',['orca','computer','get-app-state',*selector,'--restore-window','--json','--no-screenshot'])
        cmd('orca-button',['orca','computer','click',*selector,'--x','100','--y','290','--json','--no-screenshot'])
        cmd('orca-editor',['orca','computer','click',*selector,'--x','100','--y','165','--json','--no-screenshot'])
        cmd('orca-type',['orca','computer','type-text',*selector,'--text','PRODUCT_86_KEYBOARD','--json','--no-screenshot'])
        cmd('orca-after-input',['orca','computer','get-app-state',*selector,'--json'])
        doc=(OUT/'A-document.txt').read_text(encoding='utf-8-sig') if (OUT/'A-document.txt').exists() else ''
        events=(OUT/'A-events.tsv').read_text(encoding='utf-8-sig')
        checks['actual_input']='PRODUCT_86_KEYBOARD' in doc and 'button-click\t1' in events
        log('input-observed',document=doc,button_click='button-click\t1' in events)
        capture('03-input',snap(a)['rect'])
        product('sessions','app','sessions')
        result=product('release','app','release','--session',session);assert result['ok'];session=None
        after=snap(a);checks['release_restore']=all(after[k]==initial['A'][k] for k in ('rect','topmost','visible','placement'))
        log('restoration-observed',initial=initial['A'],after=after)
        activate(a);capture('04-restored',after['rect'])
        product('inventory-min','app','windows',private=True)
        result=product('too-small','app','attach','--window',str(a),'--region','220,180,180,140')
        after=snap(a);checks['minimum_rollback']=not result['ok'] and all(after[k]==initial['A'][k] for k in ('rect','topmost','visible'))
        log('minimum-rollback-observed',state=after)
        product('inventory-stale','app','windows',private=True)
        api('PostMessageW',b,0x10,0,0);procs[1].wait(timeout=8)
        result=product('stale-refusal','app','attach','--window',str(b),'--region','220,180,640,420');checks['stale_refusal']=not result['ok']
    except BaseException:
        log('failure',traceback=traceback.format_exc())
    finally:
        if session:
            try:product('cleanup-release','app','release','--session',session)
            except BaseException:log('cleanup-error',traceback=traceback.format_exc())
        for role,ident in owned.items():
            try:
                state=snap(ident['hwnd']);assert state['pid']==ident['pid'] and pathlib.Path(state['exe'])==binary
                api('PostMessageW',ident['hwnd'],0x10,0,0)
            except BaseException:pass
        for p in procs:
            try:p.wait(timeout=8)
            except subprocess.TimeoutExpired:log('cleanup-process-alive',pid=p.pid)
        if foreground:activate(foreground)
        api('SetCursorPos',cursor.x,cursor.y)
        log('cleanup',foreground_expected=foreground,foreground_actual=u.GetForegroundWindow(),cursor_expected=[cursor.x,cursor.y])
        (OUT/'summary.json').write_text(json.dumps(checks,indent=2),encoding='utf-8')
        print('EVIDENCE='+str(OUT),flush=True)
    sys.exit(0 if len(checks)==6 and all(checks.values()) else 1)
