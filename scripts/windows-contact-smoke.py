#!/usr/bin/env python3
"""Exercise Windows native pointer shortcuts in this worktree's debug app.

Run after npm run dev:build. Do not touch input during the test. The test uses
unique notes, restores the pointer, and stops only a process it started at the
exact worktree binary path. It never targets the installed app's CLI endpoint.
"""
import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / 'scripts/agent-gibbon-dev.cmd'
BINARY = ROOT / 'src-tauri/target/debug/agent-gibbon.exe'


def pids():
    literal = str(BINARY).replace("'", "''")
    command = ("@(Get-CimInstance Win32_Process -Filter \"Name = 'agent-gibbon.exe'\" | "
               f"Where-Object {{ $_.ExecutablePath -eq '{literal}' }} | "
               'ForEach-Object { $_.ProcessId }) | ConvertTo-Json -Compress')
    output = subprocess.check_output(
        ['powershell', '-NoProfile', '-Command', command], text=True, timeout=20).strip()
    value = json.loads(output) if output else []
    return set(value if isinstance(value, list) else [value])


def main():
    assert sys.platform == 'win32', 'This smoke test requires Windows'
    user = ctypes.WinDLL('user32', use_last_error=True)
    user.SetProcessDpiAwarenessContext.argtypes = [wintypes.HANDLE]
    user.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))
    user.GetForegroundWindow.restype = wintypes.HWND
    user.FindWindowW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR]
    user.FindWindowW.restype = wintypes.HWND
    user.GetDpiForWindow.argtypes = [wintypes.HWND]
    user.GetAsyncKeyState.argtypes = [ctypes.c_int]
    user.GetAsyncKeyState.restype = ctypes.c_short
    user.GetCursorPos.argtypes = [ctypes.POINTER(wintypes.POINT)]
    user.SetCursorPos.argtypes = [ctypes.c_int, ctypes.c_int]
    user.keybd_event.argtypes = [wintypes.BYTE, wintypes.BYTE, wintypes.DWORD, ctypes.c_size_t]
    keys = (0x11, 0x12, 0x5b)  # Control, Alt, left Windows
    assert all(user.GetAsyncKeyState(key) >= 0 for key in (*keys, 0x5c)), 'Release modifier keys'
    original = wintypes.POINT()
    assert user.GetCursorPos(ctypes.byref(original))
    foreground = user.GetForegroundWindow()
    before = pids()
    assert not before, 'Stop this worktree development app before running this smoke test'
    note = 'win-contact-' + uuid.uuid4().hex[:8]
    created = False
    storage = None

    def cli(action, *args):
        result = subprocess.run(
            [str(CLI), action, '--id', note, *args], cwd=ROOT,
            capture_output=True, text=True, encoding='utf-8', timeout=30)
        assert result.returncode == 0, result.stdout + result.stderr
        value = json.loads(result.stdout)
        assert value['ok'], value
        return value

    def move(x, y):
        assert user.SetCursorPos(round(x), round(y))

    def modifiers(pressed):
        for key in keys:
            if key in pressed:
                user.keybd_event(key, 0, 0, 0)
        for key in reversed(keys):
            if key not in pressed:
                user.keybd_event(key, 0, 2, 0)
        time.sleep(.08)

    def state_after(delay=.15):
        time.sleep(delay)
        value = cli('inspect')
        assert user.GetForegroundWindow() == foreground, 'Shortcut stole foreground focus'
        return value

    def point(state):
        return state['position']['x'] + 30 * scale, state['position']['y'] + 6 * scale

    try:
        opened = cli('open', '--title', 'Windows 접촉 검증', '--markdown', '# 접촉 검증 본문 보존')
        created = True
        expected = (Path(os.environ['APPDATA']) / 'org.egoing.floatnote' / 'notes'
                    / opened['instance'] / 'docs' / note / 'index.md').resolve()
        received = Path(opened['storage_path']).resolve()
        assert received == expected, ('Unexpected test storage path', received, expected)
        storage = received
        # Select a horizontal entrance direction, then wait for placement.
        move(original.x + 40, original.y)
        deadline = time.monotonic() + 6
        while cli('inspect')['animating']:
            assert time.monotonic() < deadline, 'Entrance did not finish'
            time.sleep(.1)
        initial = state_after(.3)
        handle = user.FindWindowW(None, f"Agent Gibbon [{initial['dev_label']}] — {note}")
        assert handle, 'Cannot find this test note window'
        scale = user.GetDpiForWindow(handle) / 96
        x, y = point(initial)
        outside = initial['position']['y'] - 30 * scale
        move(x, outside)
        modifiers({0x11})
        move(x, y)
        folded = state_after()
        assert folded['folded'], ('Control contact did not fold', folded)
        assert state_after(.3)['folded'], 'Contact repeatedly toggled without rearming'
        assert folded['position'] == initial['position']

        for index in range(6):
            move(x, outside)
            time.sleep(.08)
            move(x, y)
            # Shorter than the WebView's 100ms polling interval. Native input
            # sampling must preserve each contact while another app has focus.
            time.sleep(.045)
            move(x, outside)
            state = state_after(.12)
            assert state['folded'] == (index % 2 == 1), ('Contact missed', index, state)
        # A folded note accepts a 3 DIP margin, expanded notes do not.
        move(x, outside)
        time.sleep(.08)
        move(x, initial['position']['y'] - 3 * scale)
        expanded = state_after()
        assert not expanded['folded'], 'Folded margin did not unfold'
        move(x, outside)
        time.sleep(.08)
        move(x, initial['position']['y'] - 3 * scale)
        assert not state_after()['folded'], 'Expanded note accepted folded margin'
        modifiers(set())
        for key in ('position', 'size', 'focused', 'always_on_top', 'note'):
            assert expanded[key] == initial[key], (key, initial[key], expanded[key])
        print('PASS: six 45ms Ctrl contacts, repeat prevention, 4 DIP folded margin and state preservation', flush=True)

        move(x, outside)
        modifiers({0x12})
        move(x, y)
        warped = state_after()
        assert warped['position']['x'] != initial['position']['x'], ('Alt contact did not warp', warped)
        assert state_after(.3)['position'] == warped['position'], 'Warp repeated without pointer exit'
        # Re-enter with Alt still held; it must warp back once.
        next_x, next_y = point(warped)
        move(next_x, warped['position']['y'] - 30 * scale)
        time.sleep(.08)
        move(next_x, next_y)
        returned = state_after()
        assert returned['position'] == initial['position'], (initial, returned)
        modifiers(set())
        for key in ('size', 'folded', 'focused', 'always_on_top', 'note'):
            assert returned[key] == initial[key], (key, initial[key], returned[key])
        print('PASS: Alt warp, exit/reentry rearming and focus/body/size preservation', flush=True)

        move(x, outside)
        modifiers(set(keys))
        move(x, y)
        hidden = state_after()
        assert not hidden['visible'], ('Close chord did not hide', hidden)
        assert hidden['position'] == returned['position'], 'Close chord warped before hiding'
        assert hidden['folded'] == returned['folded'], 'Close chord folded before hiding'
        cli('update', '--markdown', '숨김 유지')
        assert not state_after()['visible'], 'Hidden update reopened the note'
        modifiers(set())
        print('PASS: Ctrl+Alt+Win close priority and hidden update', flush=True)
    finally:
        try:
            modifiers(set())
            move(original.x, original.y)
            if created:
                try:
                    cli('close')
                except Exception as error:
                    print(f'Cleanup close failed: {error}', file=sys.stderr)
        finally:
            # Recheck both path and PID even if input/CLI cleanup failed.
            for pid in pids() - before:
                command = (f"$p = Get-Process -Id {pid} -ErrorAction SilentlyContinue; "
                           f"if ($p -and $p.Path -eq '{str(BINARY).replace(chr(39), chr(39) * 2)}') "
                           '{ Stop-Process -InputObject $p }')
                subprocess.run(['powershell', '-NoProfile', '-Command', command], check=True, timeout=20)
            assert not pids(), 'Test process is still running; refusing storage cleanup'
            if storage is not None:
                # Only this UUID note's verified file and now-empty directory.
                storage.unlink()
                storage.parent.rmdir()


if __name__ == '__main__':
    main()
