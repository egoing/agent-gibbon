"""기존 Gibbon 인스턴스의 PID/경로/기하학만 읽는다. 제목/본문은 저장하지 않는다."""
import ctypes as c,json,pathlib,subprocess,sys,os
from ctypes import wintypes as w
root=pathlib.Path(__file__).resolve().parent
info=json.loads(subprocess.check_output(['node','scripts/dev.mjs','info'],text=True))
ps='Get-CimInstance Win32_Process -Filter "Name = \'agent-gibbon.exe\'" | Where-Object { $_.ExecutablePath -ne $env:GIBBON_TEST_BINARY } | Select-Object ProcessId,ExecutablePath | ConvertTo-Json -Compress'
raw=subprocess.check_output(['powershell.exe','-NoProfile','-Command',ps],env={**os.environ,'GIBBON_TEST_BINARY':info['binary']},text=True)
processes=json.loads(raw) if raw.strip() else []
if isinstance(processes,dict):processes=[processes]
u=c.WinDLL('user32');u.SetProcessDpiAwarenessContext(c.c_void_p(-4))
u.GetWindowThreadProcessId.argtypes=[w.HWND,c.POINTER(w.DWORD)];u.GetWindowRect.argtypes=[w.HWND,c.POINTER(w.RECT)];u.IsWindowVisible.argtypes=[w.HWND];u.GetWindowLongW.argtypes=[w.HWND,c.c_int]
cbtype=c.WINFUNCTYPE(w.BOOL,w.HWND,w.LPARAM);windows=[]
def cb(h,param):
    pid=w.DWORD();u.GetWindowThreadProcessId(h,c.byref(pid))
    if pid.value in {p['ProcessId'] for p in processes}:
        r=w.RECT();u.GetWindowRect(h,c.byref(r));windows.append(dict(hwnd=h,pid=pid.value,rect=[r.left,r.top,r.right,r.bottom],visible=bool(u.IsWindowVisible(h)),topmost=bool(u.GetWindowLongW(h,-20)&8)))
    return True
callback=cbtype(cb);u.EnumWindows(callback,0)
data=dict(processes=processes,windows=windows)
name=sys.argv[1];(root/('existing-'+name+'.json')).write_text(json.dumps(data,indent=2),encoding='utf-8')
if name=='after':
    before=json.loads((root/'existing-before.json').read_text());print(json.dumps(dict(unchanged=before==data,before_processes=len(before['processes']),after_processes=len(processes))))
else:print(json.dumps(dict(processes=len(processes),windows=len(windows))))
