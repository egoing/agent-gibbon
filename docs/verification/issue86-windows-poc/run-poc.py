"""#86 전용 프로세스 두 개만 제어하는 Windows 선행 실증. 제품 코드 아님.
Usage: python docs/verification/issue86-windows-poc/run-poc.py
증거는 새 run-* 폴더, 실행 파일은 로컬 임시 폴더에 생성한다.
"""
import ctypes as c
from ctypes import wintypes as w
import datetime, json, os, pathlib, platform, subprocess, sys, tempfile, time, traceback
from PIL import ImageGrab

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

api("SetProcessDpiAwarenessContext",-4)
procs=[]; owned={}; initial={}; foreground=u.GetForegroundWindow(); cursor=POINT(); u.GetCursorPos(c.byref(cursor))
passed={}; failure=None; binary=None
try:
    log("environment",root=str(HERE.parents[2]),source_sha=subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip(),
        initial_git_status=subprocess.check_output(["git","status","--porcelain=v1"],text=True).strip(),os=platform.platform(),python=sys.version,
        python_exe=sys.executable,original_foreground=foreground,original_cursor=[cursor.x,cursor.y],screen=[u.GetSystemMetrics(0),u.GetSystemMetrics(1)],
        monitor_count=u.GetSystemMetrics(80))
    # No elevation or global settings changes; this executable lives only in a new temp directory.
    temp=pathlib.Path(tempfile.mkdtemp(prefix="gibbon-issue86-task8f3bf6-")); binary=temp/"Issue86TestWindow.exe"
    compiler=r"C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe"
    build=cmd("compile",[compiler,"/nologo","/target:winexe","/r:System.Windows.Forms.dll","/r:System.Drawing.dll","/out:"+str(binary),str(HERE/"TestWindow.cs")])
    assert build.returncode==0,build.stdout+build.stderr
    for role in ("A","B"):
        p=subprocess.Popen([str(binary),str(OUT),role]); procs.append(p)
        ready=OUT/(role+"-ready.json")
        for _ in range(100):
            if ready.exists(): break
            if p.poll() is not None: raise RuntimeError("Dedicated window exited before readiness")
            time.sleep(.1)
        owned[role]=json.loads(ready.read_text(encoding="utf-8-sig"))
        assert owned[role]["pid"]==p.pid
        state=snap(owned[role]["hwnd"])
        assert pathlib.Path(state["exe"]).resolve()==binary.resolve() and state["pid"]==p.pid
        initial[role]=state; log("owned-initial",role=role,state=state)
    a=owned["A"]["hwnd"]; b=owned["B"]["hwnd"]
    desktop_evidence(a,b)
    cmd("orca-initial-windows",["orca","computer","list-windows","--app","pid:"+str(owned["A"]["pid"]),"--json"])
    cmd("orca-initial-focus",["orca","computer","get-app-state","--app","pid:"+str(owned["A"]["pid"]),"--window-index","0","--restore-window","--json","--no-screenshot"])
    # Each screenshot is restricted to our own window region; no desktop-wide capture.
    # Child processes may initially be behind Orca because Windows rejects activation.
    # Raise only our own ordinary windows without activation, then click the visible title.
    assert api("SetWindowPos",b,0,0,0,0,0,0x13)
    assert api("SetWindowPos",a,0,0,0,0,0,0x13)
    activate(a)
    if u.GetAncestor(u.WindowFromPoint(POINT(*point_in(a,80,15))),2)!=a:
        # Final native exposure probe; restore normal before testing the ordinary-window case.
        api("SetWindowPos",a,-1,0,0,0,0,0x13)
        time.sleep(.6)
        desktop_evidence(a,b)
        log("topmost-exposure-probe",point=list(point_in(a,80,15)),root=u.GetAncestor(u.WindowFromPoint(POINT(*point_in(a,80,15))),2))
        cmd("orca-exposure-capture",["orca","computer","get-app-state","--app","pid:"+str(owned["A"]["pid"]),"--window-index","0","--restore-window","--json"])
        cmd("orca-exposure-click",["orca","computer","click","--app","pid:"+str(owned["A"]["pid"]),"--window-index","0","--element-index","2","--json","--no-screenshot"])
        api("SetWindowPos",a,-2,0,0,0,0,0x13)
    click(*point_in(a,80,15),a)
    capture("01-initial-A",initial["A"]["rect"])
    target=[220,180,860,600]
    assert api("SetWindowPos",a,0,220,180,640,420,0x14)
    moved=snap(a); log("position-observation",requested=target,actual=moved)
    passed["position_size"]=moved["rect"]==target
    capture("02-position-A",target)
    assert api("SetWindowPos",a,-1,0,0,0,0,0x13)
    cmd("orca-B-activate",["orca","computer","get-app-state","--app","pid:"+str(owned["B"]["pid"]),"--window-index","0","--restore-window","--json","--no-screenshot"])
    activate(b)
    # Activate B using an actual click on its exposed title bar, then query foreground and overlap root.
    click(*point_in(b,80,15),b)
    time.sleep(.5)
    overlap=POINT(300,300); hit=u.GetAncestor(u.WindowFromPoint(overlap),2)
    log("topmost-observation",a=snap(a),b=snap(b),foreground=u.GetForegroundWindow(),overlap_point=[300,300],overlap_root=hit)
    passed["topmost_after_ordinary_activation"]=(u.GetForegroundWindow()==b and hit==a and snap(a)["topmost"] and not snap(b)["topmost"])
    capture("03-topmost-B-active",[150,150,950,710])
    # Official Orca tools: PID + fresh HWND selector, fresh state before coordinate actions.
    orca="orca"
    cmd("orca-windows",[orca,"computer","list-windows","--app","pid:"+str(owned["A"]["pid"]),"--json"])
    selector=["--app","pid:"+str(owned["A"]["pid"]),"--window-index","0"]
    cmd("orca-before-input",[orca,"computer","get-app-state",*selector,"--json","--no-screenshot"])
    # Native click gives the verified text receiver foreground focus before trying official input.
    click(*point_in(owned["A"]["editor"],50,30),a)
    cmd("orca-focused",[orca,"computer","get-app-state",*selector,"--json","--no-screenshot"])
    cmd("orca-click",[orca,"computer","click",*selector,"--x","100","--y","165","--json","--no-screenshot"])
    cmd("orca-type",[orca,"computer","type-text",*selector,"--text","ORCA_86_OK ","--json","--no-screenshot"])
    cmd("orca-after-input",[orca,"computer","get-app-state",*selector,"--json","--no-screenshot"])
    time.sleep(.4)
    doc=OUT/"A-document.txt"
    orca_text=doc.read_text(encoding="utf-8-sig") if doc.exists() else ""
    log("orca-document-observation",document=orca_text,contains_expected="ORCA_86_OK" in orca_text)
    capture("04-orca-input",target)
    # Independent SendInput click+keyboard with application event/document evidence.
    click(*point_in(owned["A"]["button"],55,20),a)
    click(*point_in(owned["A"]["editor"],50,50),a)
    keytext("NATIVE_86_OK",a)
    actual_text=doc.read_text(encoding="utf-8-sig")
    events=(OUT/"A-events.tsv").read_text(encoding="utf-8-sig")
    passed["real_click_keyboard"]="NATIVE_86_OK" in actual_text and "button-click\t1" in events and "editor-keypress" in events and "editor-mousedown" in events
    log("native-input-observation",document=actual_text,button_click_observed="button-click\t1" in events,keypress_observed="editor-keypress" in events)
    capture("05-native-input",target)
    # Empirically show app-defined minimum constrains arbitrary requested rectangles.
    assert api("SetWindowPos",a,0,220,180,100,80,0x14)
    log("minimum-size-observation",requested=[220,180,320,260],actual=snap(a))
    assert api("SetWindowPos",a,0,220,180,640,420,0x14)
    # Release TOPMOST: ordinary B should now obscure A at the same overlap point.
    assert api("SetWindowPos",a,-2,0,0,0,0,0x13)
    click(*point_in(b,80,15),b)
    time.sleep(.4)
    released=snap(a); released_hit=u.GetAncestor(u.WindowFromPoint(overlap),2)
    log("release-observation",a=released,b=snap(b),foreground=u.GetForegroundWindow(),overlap_root=released_hit)
    release_ok=not released["topmost"] and released_hit==b and u.GetForegroundWindow()==b
    capture("06-release-B-above",[150,150,950,710])
    restored=restore(a,initial["A"])
    assert api("SetWindowPos",a,0,0,0,0,0,0x13)
    click(*point_in(a,80,15),a)
    capture("07-restored-A",initial["A"]["rect"])
    passed["release_restore"]=release_ok and all(restored[x]==initial["A"][x] for x in ("rect","placement","topmost","visible"))
except BaseException:
    failure=traceback.format_exc(); log("failure",traceback=failure)
finally:
    # Ownership is rechecked before restoring/closing; no process-name kill or user-window mutation.
    for role, identity in owned.items():
        try:
            state=snap(identity["hwnd"])
            assert state["pid"]==identity["pid"] and pathlib.Path(state["exe"]).resolve()==binary.resolve()
            if role in initial: restore(identity["hwnd"],initial[role])
            api("PostMessageW",identity["hwnd"],0x10,0,0)
        except BaseException: log("cleanup-error",role=role,traceback=traceback.format_exc())
    for p in procs:
        try: p.wait(timeout=8)
        except subprocess.TimeoutExpired: log("cleanup-process-still-running",pid=p.pid)
    if foreground: api("SetForegroundWindow",foreground)
    api("SetCursorPos",cursor.x,cursor.y)
    now=POINT(); u.GetCursorPos(c.byref(now))
    log("cleanup-observation",processes=[dict(pid=p.pid,exit=p.poll()) for p in procs],
        foreground_expected=foreground,foreground_actual=u.GetForegroundWindow(),cursor_expected=[cursor.x,cursor.y],cursor_actual=[now.x,now.y])
    summary=dict(checks=passed,gate="PASS" if len(passed)==4 and all(passed.values()) and not failure else "CLOSED",failure=failure)
    (OUT/"summary.json").write_text(json.dumps(summary,indent=2,ensure_ascii=False),encoding="utf-8")
    log("summary",**summary)
    print("EVIDENCE="+str(OUT),flush=True)
sys.exit(0 if summary["gate"]=="PASS" else 1)
