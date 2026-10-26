"""Windows native alpha/focus regression. --interact moves the cursor/clicks own note.

Requires a built dev binary and no running dev app in this worktree. Cleans up
only its own process and UUID note directories. Installed apps are untouched.
"""
import argparse
import ctypes as C
from ctypes import wintypes as W
import json
from pathlib import Path
import subprocess
import shutil
import time
import uuid
from windows_uia import control_center

parser = argparse.ArgumentParser()
parser.add_argument('--interact', action='store_true')
args = parser.parse_args()
ROOT = Path(__file__).resolve().parents[1]
binary = ROOT / 'src-tauri/target/debug/agent-gibbon.exe'
kernel = C.WinDLL('kernel32', use_last_error=True)
kernel.OpenProcess.argtypes = [W.DWORD, W.BOOL, W.DWORD]
kernel.OpenProcess.restype = W.HANDLE
kernel.QueryFullProcessImageNameW.argtypes = [W.HANDLE, W.DWORD, W.LPWSTR, C.POINTER(W.DWORD)]
kernel.TerminateProcess.argtypes = [W.HANDLE, W.UINT]
kernel.WaitForSingleObject.argtypes = [W.HANDLE, W.DWORD]
kernel.CloseHandle.argtypes = [W.HANDLE]
user = C.WinDLL('user32', use_last_error=True)
user.SetProcessDpiAwarenessContext.argtypes = [C.c_void_p]
user.SetProcessDpiAwarenessContext(C.c_void_p(-4))
user.GetForegroundWindow.restype = W.HWND
user.GetWindowThreadProcessId.argtypes = [W.HWND, C.POINTER(W.DWORD)]
user.FindWindowW.argtypes = [W.LPCWSTR, W.LPCWSTR]
user.FindWindowW.restype = W.HWND
user.GetLayeredWindowAttributes.argtypes = [W.HWND, C.POINTER(W.DWORD), C.POINTER(C.c_ubyte), C.POINTER(W.DWORD)]
user.GetWindowLongW.argtypes = [W.HWND, C.c_int]
user.GetWindowLongW.restype = C.c_long
user.GetAncestor.argtypes = [W.HWND, W.UINT]
user.GetAncestor.restype = W.HWND
user.WindowFromPoint.argtypes = [W.POINT]
user.WindowFromPoint.restype = W.HWND
user.SetForegroundWindow.argtypes = [W.HWND]
user.GetDpiForWindow.argtypes = [W.HWND]
user.GetDpiForWindow.restype = W.UINT
user.GetCursorPos.argtypes = [C.POINTER(W.POINT)]
user.GetAsyncKeyState.argtypes = [C.c_int]
user.GetAsyncKeyState.restype = C.c_short
user.GetWindowTextW.argtypes = [W.HWND, W.LPWSTR, C.c_int]
user.GetClientRect.argtypes = [W.HWND, C.POINTER(W.RECT)]
user.ClientToScreen.argtypes = [W.HWND, C.POINTER(W.POINT)]
original_foreground = user.GetForegroundWindow()
original_cursor = W.POINT()
assert user.GetCursorPos(C.byref(original_cursor))
ids = ['visual-' + uuid.uuid4().hex[:8], 'visual-peer-' + uuid.uuid4().hex[:8]]
handles = []
storage_paths = set()
owned_process = None


def cli(*values):
    result = subprocess.run(
        ['cmd.exe', '/d', '/c', str(ROOT / 'scripts/agent-gibbon-dev.cmd'), *values],
        cwd=ROOT, capture_output=True, encoding='utf-8', timeout=35,
    )
    assert result.returncode == 0, (result.stdout, result.stderr)
    data = json.loads(result.stdout)
    assert data['ok'], data
    if data.get('id') in ids and data.get('storage_path'):
        storage_paths.add(Path(data['storage_path']))
    return data


def idle_keys():
    assert not any(user.GetAsyncKeyState(key) & 0x8000 for key in (16, 17, 18, 91, 92, 1, 2)), '사용자 입력 중: 검증 중단'


def native(state, check_alpha=True):
    found = []
    callback = C.WINFUNCTYPE(W.BOOL, W.HWND, W.LPARAM)

    @callback
    def visit(handle, _):
        text = C.create_unicode_buffer(512)
        user.GetWindowTextW(handle, text, 512)
        if text.value.endswith(' — ' + state['id']):
            found.append(handle)
        return True

    user.EnumWindows(visit, 0)
    assert len(found) == 1, found
    hwnd = found[0]
    if hwnd not in handles:
        handles.append(hwnd)
    if not check_alpha:
        return hwnd
    key, alpha, flags = W.DWORD(), C.c_ubyte(), W.DWORD()
    assert user.GetLayeredWindowAttributes(hwnd, C.byref(key), C.byref(alpha), C.byref(flags)), C.get_last_error()
    assert alpha.value == int(state['opacity'] * 255 + .5), (alpha.value, state)
    assert flags.value == 2
    assert user.GetWindowLongW(hwnd, -20) & 0x80000
    assert not user.GetWindowLongW(hwnd, -20) & 0x8000000, '표시 후 클릭 활성화 복구 실패'
    return hwnd


def stable_focus():
    assert user.GetForegroundWindow() == original_foreground, '외부 foreground 변경'


def move(x, y):
    idle_keys()
    assert user.SetCursorPos(int(x), int(y))
    time.sleep(.45)


try:
    idle_keys()
    probe = subprocess.run(['cmd.exe', '/d', '/c', str(ROOT / 'scripts/agent-gibbon-dev.cmd'), 'list'], cwd=ROOT, capture_output=True, timeout=35)
    assert probe.returncode != 0, '이 워크트리 개발 앱을 먼저 종료하세요. 기존 앱은 검증에서 종료하지 않습니다.'
    state = cli('open', '--id', ids[0], '--markdown', '# Visual alpha test')
    assert state['visible'] and not state['focused']
    # Register ownership before alpha assertions so a regression still cleans up.
    hwnd = native(state, check_alpha=False)
    pid = W.DWORD()
    user.GetWindowThreadProcessId(hwnd, C.byref(pid))
    process = kernel.OpenProcess(0x100001 | 0x1000, False, pid.value)
    assert process, C.get_last_error()
    image_path, length = C.create_unicode_buffer(32768), W.DWORD(32768)
    assert kernel.QueryFullProcessImageNameW(process, 0, image_path, C.byref(length))
    assert Path(image_path.value).resolve() == binary.resolve(), image_path.value
    owned_process = process
    native(state)
    stable_focus()
    peer = cli('open', '--id', ids[1], '--markdown', 'Independent note')
    native(peer)
    stable_focus()
    cli('close', '--id', ids[1])
    for _ in range(3):
        state = cli('close', '--id', ids[0])
        assert not state['visible']
        native(state)
        hidden = cli('update', '--id', ids[0], '--status', 'hidden')
        assert not hidden['visible'] and hidden['position'] == state['position']
        stable_focus()
        state = cli('open', '--id', ids[0])
        assert state['visible'] and not state['focused']
        native(state)
        stable_focus()
    if args.interact:
        # Move right to complete the entrance; this affects only pointer position.
        move(original_cursor.x + 32, original_cursor.y)
        time.sleep(1.1)
        state = cli('inspect', '--id', ids[0])
        assert not state['animating'], state
        assert state['opacity'] == .5, state
        native(state)
        before = state['position']
        state = cli('update', '--id', ids[0], '--markdown', 'Feedback test')
        assert state['feedback_active'] and state['opacity'] == 1, state
        assert state['position'] == before
        native(state)
        stable_focus()
        time.sleep(3.2)
        # Read HWND directly before inspect (which itself synchronizes alpha).
        native({**state, 'opacity': .5})
        state = cli('inspect', '--id', ids[0])
        assert not state['feedback_active'] and state['opacity'] == .5, state
        native(state)
        p, size = state['position'], state['size']
        move(p['x'] + size['width'] / 2, p['y'] + size['height'] / 2)
        native({**state, 'opacity': 1})
        state = cli('inspect', '--id', ids[0])
        assert state['opacity'] == 1, state
        native(state)
        stable_focus()
        move(*control_center(hwnd, '메모 접기'))
        point = W.POINT()
        user.GetCursorPos(C.byref(point))
        assert user.GetAncestor(user.WindowFromPoint(point), 2) == hwnd, '다른 창 위 입력 방지'
        idle_keys()
        user.mouse_event(2, 0, 0, 0, 0)
        user.mouse_event(4, 0, 0, 0, 0)
        time.sleep(.4)
        state = cli('inspect', '--id', ids[0])
        assert state['folded'], state
        move(original_cursor.x, original_cursor.y)
        native({**state, 'opacity': .8})
        state = cli('inspect', '--id', ids[0])
        assert state['opacity'] == .8, state
        native(state)
    print(json.dumps({'ok': True, 'checks': ['native alpha matches inspect', 'activation restored after show', 'new window keeps foreground', 'three hide/show cycles', 'hidden updates preserve position'], 'interactive': args.interact, 'ids': ids}))
finally:
    if args.interact:
        user.SetCursorPos(original_cursor.x, original_cursor.y)
        if user.GetForegroundWindow() in handles:
            user.SetForegroundWindow(original_foreground)
    for note_id in ids:
        try:
            cli('close', '--id', note_id)
        except Exception as error:
            print('정리 오류:', error)
    if owned_process:
        assert kernel.TerminateProcess(owned_process, 0)
        assert kernel.WaitForSingleObject(owned_process, 10000) == 0
        kernel.CloseHandle(owned_process)
        for path in storage_paths:
            # Validate each exact file and parent before recursively removing it.
            path = path.resolve()
            assert path.name == 'index.md' and path.parent.name in ids, path
            assert path.parent.parent.name == 'docs', path
            assert path.parent.parent.parent.name == state['instance'], path
            shutil.rmtree(path.parent)
