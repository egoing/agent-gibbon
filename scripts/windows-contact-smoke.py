#!/usr/bin/env python3
"""Exercise Windows native pointer shortcuts in this worktree's debug app.

Run after npm run dev:build. Do not touch input during the test. The test uses
unique notes, restores the pointer, and stops only a process it started at the
exact worktree binary path. It never targets the installed app's CLI endpoint.
Pointer shortcuts require a nonfocused note and an observed outside-to-inside
entry with the keys already held. Pressing a key while inside never triggers.
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
from windows_uia import control_rect

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
    user.SetForegroundWindow.argtypes = [wintypes.HWND]
    user.FindWindowW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR]
    user.FindWindowW.restype = wintypes.HWND
    user.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
    user.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
    user.WindowFromPoint.argtypes = [wintypes.POINT]
    user.WindowFromPoint.restype = wintypes.HWND
    user.GetAncestor.argtypes = [wintypes.HWND, wintypes.UINT]
    user.GetAncestor.restype = wintypes.HWND
    user.GetDpiForWindow.argtypes = [wintypes.HWND]
    user.GetAsyncKeyState.argtypes = [ctypes.c_int]
    user.GetAsyncKeyState.restype = ctypes.c_short
    user.GetCursorPos.argtypes = [ctypes.POINTER(wintypes.POINT)]
    user.SetCursorPos.argtypes = [ctypes.c_int, ctypes.c_int]
    user.keybd_event.argtypes = [wintypes.BYTE, wintypes.BYTE, wintypes.DWORD, ctypes.c_size_t]
    user.mouse_event.argtypes = [wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, ctypes.c_size_t]
    keys = (0x11, 0x12, 0x5b)  # Control, Alt, left Windows
    assert all(user.GetAsyncKeyState(key) >= 0 for key in (*keys, 0x5c)), 'Release modifier keys'
    original = wintypes.POINT()
    assert user.GetCursorPos(ctypes.byref(original))
    foreground = user.GetForegroundWindow()
    original_foreground = foreground
    before = pids()
    assert not before, 'Stop this worktree development app before running this smoke test'
    note = 'win-contact-' + uuid.uuid4().hex[:8]
    created = False
    storage = None
    handle = None
    owned_pid = None
    title = None

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

    def click_control(name):
        def guard():
            actual_pid = wintypes.DWORD()
            user.GetWindowThreadProcessId(handle, ctypes.byref(actual_pid))
            actual_title = ctypes.create_unicode_buffer(512)
            user.GetWindowTextW(handle, actual_title, len(actual_title))
            assert actual_pid.value == owned_pid and actual_title.value == title, 'Test HWND identity changed'
            assert user.GetForegroundWindow() == handle, 'Foreground changed before control click'
            assert not any(user.GetAsyncKeyState(key) < 0 for key in (0x10, *keys, 0x5c, 1, 2, 4)), 'Release input before control click'
        guard()
        rect = control_rect(handle, name)  # Read-only UIA lookup of this named control.
        move(rect['x'] + rect['width'] / 2, rect['y'] + rect['height'] / 2)
        actual = wintypes.POINT()
        assert user.GetCursorPos(ctypes.byref(actual))
        assert user.GetAncestor(user.WindowFromPoint(actual), 2) == handle, 'Control click would hit another window'
        guard()
        try:
            user.mouse_event(2, 0, 0, 0, 0)
        finally:
            user.mouse_event(4, 0, 0, 0, 0)

    def wait_fold(expected):
        deadline = time.monotonic() + 3
        while True:
            state = state_after(.1)
            if state['folded'] == expected:
                return state
            assert time.monotonic() < deadline, ('Button did not change native fold state', expected, state)

    def accessible_control(name, invoke=False, focus=False):
        # Query only this test HWND's accessibility subtree. Scrollbars can
        # shift the toolbar, so guessing a button coordinate is insufficient.
        literal = name.replace("'", "''")
        invoke_action = '$element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke(); ' if invoke else ''
        focus_action = '$element.SetFocus(); ' if focus else ''
        command = (
            "$ErrorActionPreference = 'Stop'; "
            'Add-Type -AssemblyName UIAutomationClient; '
            'Add-Type -AssemblyName UIAutomationTypes; '
            f'$root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]{handle}); '
            '$condition = [System.Windows.Automation.PropertyCondition]::new('
            f"[System.Windows.Automation.AutomationElement]::NameProperty, '{literal}'); "
            '$element = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition); '
            "if ($null -eq $element) { throw 'Test control not found' }; "
            f'{focus_action}'
            '$rect = $element.Current.BoundingRectangle; '
            '$before = @{x=$rect.X; y=$rect.Y; width=$rect.Width; height=$rect.Height; '
            'offscreen=$element.Current.IsOffscreen}; '
            f'{invoke_action}'
            '$before | ConvertTo-Json -Compress')
        result = subprocess.run(['powershell', '-NoProfile', '-Command', command],
                                capture_output=True, encoding='utf-8', timeout=20)
        assert result.returncode == 0, result.stderr
        rect = json.loads(result.stdout)
        assert not rect['offscreen'] and rect['width'] > 0 and rect['height'] > 0, rect
        return rect

    try:
        opened = cli('open', '--title', 'Windows 접촉 검증', '--markdown', '# 접촉 검증 본문 보존')
        created = True
        own_processes = pids()
        assert len(own_processes) == 1, ('Cannot identify test process', own_processes)
        owned_pid = own_processes.pop()
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
        title = f"Agent Gibbon [{initial['dev_label']}] — {note}"
        handle = user.FindWindowW(None, title)
        assert handle, 'Cannot find this test note window'
        scale = user.GetDpiForWindow(handle) / 96
        x, y = point(initial)
        outside = initial['position']['y'] - 30 * scale
        # Enter without Control, then press it inside: key changes are not entries.
        move(x, outside)
        time.sleep(.08)
        move(x, y)
        time.sleep(.08)
        modifiers({0x11})
        assert not state_after()['folded'], 'Pressing Control inside counted as pointer entry'
        modifiers(set())
        modifiers({0x11})
        assert not state_after()['folded'], 'Control release/repress inside rearmed contact'
        move(x, outside)
        time.sleep(.08)
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

        # Alt also needs pointer entry; pressing/repressing it inside is inert.
        move(x, y)
        time.sleep(.08)
        modifiers({0x12})
        assert state_after()['position'] == initial['position'], 'Pressing Alt inside warped the note'
        modifiers(set())
        modifiers({0x12})
        assert state_after()['position'] == initial['position'], 'Alt release/repress inside rearmed contact'
        move(x, outside)
        time.sleep(.08)
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

        # Completing the close chord inside must not hide, fold, or warp.
        move(x, y)
        time.sleep(.08)
        modifiers(set(keys))
        chord_inside = state_after()
        assert chord_inside['visible'], 'Completing the close chord inside hid the note'
        assert chord_inside['position'] == returned['position']
        assert chord_inside['folded'] == returned['folded']
        move(x, outside)
        time.sleep(.08)
        move(x, y)
        hidden = state_after()
        assert not hidden['visible'], ('Close chord did not hide', hidden)
        assert hidden['position'] == returned['position'], 'Close chord warped before hiding'
        assert hidden['folded'] == returned['folded'], 'Close chord folded before hiding'
        cli('update', '--markdown', '숨김 유지')
        assert not state_after()['visible'], 'Hidden update reopened the note'
        modifiers(set())
        print('PASS: Ctrl+Alt+Win close priority and hidden update', flush=True)

        reopen_anchor = wintypes.POINT()
        assert user.GetCursorPos(ctypes.byref(reopen_anchor))
        cli('open')
        move(reopen_anchor.x + 40, reopen_anchor.y)
        deadline = time.monotonic() + 6
        while cli('inspect')['animating']:
            assert time.monotonic() < deadline, 'Reopened entrance did not finish'
            time.sleep(.1)
        current = cli('inspect')
        # Activate only this test note's own verified pencil control.
        assert user.GetForegroundWindow() == original_foreground
        accessible_control('메모 편집', invoke=True)
        accessible_control('Markdown 원문', focus=True)
        assert user.GetForegroundWindow() == handle, 'Test editor did not receive focus'
        # Move into the actual textarea before pressing Control.
        textarea = accessible_control('Markdown 원문')
        move(textarea['x'] + textarea['width'] / 2, textarea['y'] + textarea['height'] / 2)
        foreground = handle
        modifiers({0x11})
        assert not state_after(.2)['folded'], 'Editor Ctrl key folded the note'
        user.keybd_event(0x41, 0, 0, 0)  # Ctrl+A in the real textarea
        user.keybd_event(0x41, 0, 2, 0)
        assert not state_after()['folded'], 'Editor selection folded the note'
        revision = current['note']['revision']
        user.keybd_event(0x53, 0, 0, 0)  # Keep Control held after Ctrl+S
        user.keybd_event(0x53, 0, 2, 0)
        saved = state_after(.3)
        assert saved['note']['revision'] > revision, 'Ctrl+S did not save from the editor'
        assert saved['note']['markdown'] == current['note']['markdown']
        assert not saved['folded'], 'Leaving editor with Ctrl held folded the note'
        modifiers(set())
        modifiers({0x11})
        assert not state_after()['folded'], 'Ctrl release/repress inside folded the focused note'
        modifiers(set())
        print('PASS: focused editor Ctrl+A/Ctrl+S, held-Ctrl save and no inside-key rearming', flush=True)

        # Focus suppression applies to every pointer shortcut after editing too.
        x, y = point(saved)
        outside = saved['position']['y'] - 30 * scale
        for pressed, label in (({0x11}, 'Control'), ({0x12}, 'Alt'), (set(keys), 'close chord')):
            move(x, outside)
            modifiers(pressed)
            move(x, y)
            focused = state_after(.2)
            assert focused['visible'] and not focused['folded'], (label, focused)
            assert focused['position'] == saved['position'], (label, focused)
            assert focused['note'] == saved['note'], (label, focused)
            modifiers(set())
        # Screen buttons remain available even though focused contacts are ignored.
        click_control('메모 접기')
        folded_by_button = wait_fold(True)
        assert folded_by_button['position'] == saved['position'] and folded_by_button['note'] == saved['note']
        click_control('메모 펼치기')
        restored = wait_fold(False)
        assert not restored['folded'] and restored['size'] == saved['size']
        print('PASS: all focused pointer shortcuts ignored; explicit fold/unfold buttons work', flush=True)

        # Losing focus with the pointer inside is not a new contact either.
        x, y = point(restored)
        outside = restored['position']['y'] - 30 * scale
        move(x, y)
        modifiers({0x11})
        assert not state_after()['folded']
        user.SetForegroundWindow(original_foreground)
        deadline = time.monotonic() + 2
        while user.GetForegroundWindow() != original_foreground:
            assert time.monotonic() < deadline, 'Could not restore foreground for entry test'
            time.sleep(.05)
        foreground = original_foreground
        assert not state_after()['folded'], 'Focus loss inside counted as pointer entry'
        modifiers(set())
        modifiers({0x11})
        assert not state_after()['folded'], 'Key release/repress inside rearmed after focus loss'
        move(x, outside)
        time.sleep(.08)
        move(x, y)
        reentered = state_after()
        assert reentered['folded'], 'Real nonfocused Control reentry did not fold'
        assert reentered['position'] == restored['position'] and reentered['note'] == restored['note']
        modifiers(set())
        print('PASS: focus loss and key changes inside are inert; nonfocused pointer reentry rearms', flush=True)
    finally:
        try:
            modifiers(set())
            move(original.x, original.y)
            if user.GetForegroundWindow() == handle:
                user.SetForegroundWindow(original_foreground)
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
