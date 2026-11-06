"""정확한 자기 개발 바이너리 경로의 단일 프로세스에만 WM_QUIT를 보낸다."""
import ctypes as c,json,subprocess,os,time
from ctypes import wintypes as w
info=json.loads(subprocess.check_output(['node','scripts/dev.mjs','info'],text=True))
script='Get-CimInstance Win32_Process -Filter "Name = \'agent-gibbon.exe\'" | Where-Object { $_.ExecutablePath -eq $env:GIBBON_TEST_BINARY } | Select-Object ProcessId,ExecutablePath | ConvertTo-Json -Compress'
raw=subprocess.check_output(['powershell.exe','-NoProfile','-Command',script],env={**os.environ,'GIBBON_TEST_BINARY':info['binary']},text=True)
if not raw.strip():print('own server absent');raise SystemExit(0)
proc=json.loads(raw);assert isinstance(proc,dict) and proc['ExecutablePath'].lower()==info['binary'].lower()
u=c.WinDLL('user32');u.GetWindowThreadProcessId.argtypes=[w.HWND,c.POINTER(w.DWORD)];u.GetWindowThreadProcessId.restype=w.DWORD;u.PostThreadMessageW.argtypes=[w.DWORD,w.UINT,w.WPARAM,w.LPARAM]
cbtype=c.WINFUNCTYPE(w.BOOL,w.HWND,w.LPARAM)
def cb(h,param):
    pid=w.DWORD();tid=u.GetWindowThreadProcessId(h,c.byref(pid))
    if pid.value==proc['ProcessId']:print(json.dumps(dict(process=proc,tid=tid,wm_quit_return=int(u.PostThreadMessageW(tid,0x12,0,0)))));return False
    return True
callback=cbtype(cb);u.EnumWindows(callback,0);time.sleep(.6)
