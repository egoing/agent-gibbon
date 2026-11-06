"""읽기 전용: 성공 실행 전후의 원래 foreground 창 상태를 대조한다."""
import ctypes as c
from ctypes import wintypes as w
import json, pathlib, sys
root=pathlib.Path(sys.argv[1]).resolve()
rows=[json.loads(line) for line in (root/"raw.jsonl").read_text(encoding="utf-8").splitlines()]
before=next(r["state"] for r in rows if r["kind"]=="desktop-observation" and r["label"]=="original-foreground")
u=c.WinDLL("user32"); u.SetProcessDpiAwarenessContext(c.c_void_p(-4))
class RECT(c.Structure): _fields_=[(x,w.LONG) for x in ("left","top","right","bottom")]
class POINT(c.Structure): _fields_=[("x",w.LONG),("y",w.LONG)]
class WP(c.Structure): _fields_=[("length",w.UINT),("flags",w.UINT),("showCmd",w.UINT),("min",POINT),("max",POINT),("normal",RECT)]
u.GetWindowRect.argtypes=[w.HWND,c.POINTER(RECT)]
u.GetWindowPlacement.argtypes=[w.HWND,c.POINTER(WP)]
u.GetWindowLongW.argtypes=[w.HWND,c.c_int]
u.IsWindowVisible.argtypes=[w.HWND]
u.GetForegroundWindow.restype=w.HWND
h=before["hwnd"]; rect=RECT(); place=WP(); place.length=c.sizeof(place)
assert u.GetWindowRect(h,c.byref(rect)) and u.GetWindowPlacement(h,c.byref(place))
after=dict(rect=[rect.left,rect.top,rect.right,rect.bottom],placement_hex=bytes(place).hex(),topmost=bool(u.GetWindowLongW(h,-20)&8),visible=bool(u.IsWindowVisible(h)))
result=dict(before=before,after=after,foreground=u.GetForegroundWindow(),equal=all(after[key]==before[key] for key in after))
(root/"original-window-preservation.json").write_text(json.dumps(result,indent=2),encoding="utf-8")
print(json.dumps(result))
assert result["equal"]
