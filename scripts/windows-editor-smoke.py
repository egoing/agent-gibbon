"""Real Windows edit/save and edge-drag regression on this built dev worktree.

Run only in an assigned GUI test slot: python scripts/windows-editor-smoke.py --interact
Requires an idle keyboard/mouse and no running app from this worktree. Uses only
stdlib ctypes and the development CLI; never changes the clipboard or other apps.
"""
import argparse
import ctypes as C
from ctypes import wintypes as W
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'src-tauri/target/debug/agent-gibbon.exe'
CLI = ROOT / 'scripts/agent-gibbon-dev.cmd'


def processes():
    result = subprocess.run(['powershell', '-NoProfile', '-Command',
        "@(Get-CimInstance Win32_Process -Filter \"Name = 'agent-gibbon.exe'\" | "
        "Where-Object { $_.ExecutablePath -eq $env:GIBBON_EDITOR_BINARY } | "
        'ForEach-Object { $_.ProcessId }) | ConvertTo-Json -Compress'],
        env={**os.environ, 'GIBBON_EDITOR_BINARY': str(BINARY)},
        capture_output=True, text=True, timeout=20, check=True)
    value = json.loads(result.stdout) if result.stdout.strip() else []
    return set(value if isinstance(value, list) else [value])


def canonical(path):
    return os.path.normcase(os.path.realpath(path)).removeprefix('\\\\?\\')


class Keyboard(C.Structure):
    _fields_ = [('vk', W.WORD), ('scan', W.WORD), ('flags', W.DWORD),
                ('time', W.DWORD), ('extra', C.c_size_t)]


class Mouse(C.Structure):
    _fields_ = [('dx', W.LONG), ('dy', W.LONG), ('data', W.DWORD),
                ('flags', W.DWORD), ('time', W.DWORD), ('extra', C.c_size_t)]


class Hardware(C.Structure):
    _fields_ = [('message', W.DWORD), ('low', W.WORD), ('high', W.WORD)]


class Payload(C.Union):
    _fields_ = [('keyboard', Keyboard), ('mouse', Mouse), ('hardware', Hardware)]


class Input(C.Structure):
    _fields_ = [('kind', W.DWORD), ('payload', Payload)]


class MonitorInfo(C.Structure):
    _fields_ = [('size', W.DWORD), ('monitor', W.RECT), ('work', W.RECT), ('flags', W.DWORD)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--interact', action='store_true', required=True,
                        help='명시적으로 자기 노트에 마우스/키보드 입력을 허용')
    parser.parse_args()
    assert sys.platform == 'win32', 'Windows 전용 검증입니다.'
    assert BINARY.is_file(), '먼저 npm run dev:build를 실행하세요.'
    assert not processes(), '이 워크트리 개발 앱이 실행 중입니다. 중단하고 별도 검증 워크트리를 사용하세요.'
    user = C.WinDLL('user32', use_last_error=True)
    user.SetProcessDpiAwarenessContext.argtypes = [C.c_void_p]
    user.SetProcessDpiAwarenessContext(C.c_void_p(-4))
    user.GetForegroundWindow.restype = W.HWND
    user.SetForegroundWindow.argtypes = [W.HWND]
    user.GetWindowThreadProcessId.argtypes = [W.HWND, C.POINTER(W.DWORD)]
    user.FindWindowW.argtypes = [W.LPCWSTR, W.LPCWSTR]
    user.FindWindowW.restype = W.HWND
    user.GetWindowTextW.argtypes = [W.HWND, W.LPWSTR, C.c_int]
    user.GetAsyncKeyState.argtypes = [C.c_int]
    user.GetAsyncKeyState.restype = C.c_short
    user.GetCursorPos.argtypes = [C.POINTER(W.POINT)]
    user.SetCursorPos.argtypes = [C.c_int, C.c_int]
    user.GetDpiForWindow.argtypes = [W.HWND]
    user.GetDpiForWindow.restype = W.UINT
    user.WindowFromPoint.argtypes = [W.POINT]
    user.WindowFromPoint.restype = W.HWND
    user.GetAncestor.argtypes = [W.HWND, W.UINT]
    user.GetAncestor.restype = W.HWND
    user.MonitorFromWindow.argtypes = [W.HWND, W.DWORD]
    user.MonitorFromWindow.restype = W.HANDLE
    user.GetMonitorInfoW.argtypes = [W.HANDLE, C.POINTER(MonitorInfo)]
    user.SendInput.argtypes = [W.UINT, C.POINTER(Input), C.c_int]
    user.SendInput.restype = W.UINT
    user.mouse_event.argtypes = [W.DWORD, W.DWORD, W.DWORD, W.DWORD, C.c_size_t]
    watched = (0x10, 0x11, 0x12, 0x5b, 0x5c, 1, 2, 4)
    assert not any(user.GetAsyncKeyState(key) < 0 for key in watched), '먼저 보조키와 마우스 버튼을 놓으세요.'
    original_foreground = user.GetForegroundWindow()
    original_cursor = W.POINT()
    assert user.GetCursorPos(C.byref(original_cursor))
    note_id = 'win-editor-' + uuid.uuid4().hex
    temporary = Path(tempfile.mkdtemp(prefix='.windows-editor-', dir=ROOT))
    source = temporary / '한글 공백 원본.md'
    original_body = '# 편집 전\n' + '\n'.join(f'원본 행 {i}' for i in range(12)) + '\n'
    replacement = '# 실제 UI 저장\n' + '\n'.join(f'저장한 행 {i}' for i in range(8)) + '\n'
    source.write_text(original_body, encoding='utf-8')
    storage = None
    owned_pid = None
    hwnd = None
    injected_keys = set()
    mouse_down = False
    title = None

    def cli(action, *values):
        nonlocal storage
        result = subprocess.run(['cmd.exe', '/d', '/c', str(CLI), action, '--id', note_id, *values],
                                cwd=ROOT, capture_output=True, encoding='utf-8', timeout=35)
        assert result.returncode == 0, (result.stdout, result.stderr)
        state = json.loads(result.stdout)
        assert state['ok'], state
        if 'storage_path' in state:
            identifier = json.loads((ROOT / 'src-tauri/tauri.conf.json').read_text(encoding='utf-8'))['identifier']
            expected = Path(os.environ['APPDATA']) / identifier / 'notes' / state['instance'] / 'docs' / note_id / 'index.md'
            assert canonical(state['storage_path']) == canonical(expected), '예상 밖 저장 경로'
            storage = expected
        return state

    def identity():
        assert hwnd and owned_pid, '자기 노트가 아직 준비되지 않았습니다.'
        pid = W.DWORD()
        user.GetWindowThreadProcessId(hwnd, C.byref(pid))
        text = C.create_unicode_buffer(512)
        user.GetWindowTextW(hwnd, text, len(text))
        assert pid.value == owned_pid and text.value == title, '입력 대상 HWND/PID/제목 변경'

    def guard():
        identity()
        assert user.GetForegroundWindow() == hwnd, '사용자 foreground 변경: 입력 중단'
        assert not any(user.GetAsyncKeyState(key) < 0 for key in watched
                       if key not in injected_keys and not (key == 1 and mouse_down)), '사용자 입력 감지: 중단'

    def move(x, y):
        guard()
        assert user.SetCursorPos(round(x), round(y))

    def hit(x, y):
        move(x, y)
        point = W.POINT()
        assert user.GetCursorPos(C.byref(point))
        assert user.GetAncestor(user.WindowFromPoint(point), 2) == hwnd, '마우스 대상이 자기 노트가 아닙니다.'

    def click(x, y, activate_from=None):
        nonlocal mouse_down
        if activate_from is None:
            hit(x, y)
            guard()
        else:
            # Windows may reject SetForegroundWindow from an agent terminal.
            # A real click on the verified own control activates it naturally.
            identity()
            assert user.GetForegroundWindow() == activate_from, '사용자 foreground 변경: 활성화 중단'
            assert not any(user.GetAsyncKeyState(key) < 0 for key in watched), '사용자 입력 감지'
            assert user.SetCursorPos(round(x), round(y))
            point = W.POINT()
            assert user.GetCursorPos(C.byref(point))
            assert user.GetAncestor(user.WindowFromPoint(point), 2) == hwnd, '활성화 클릭 대상 변경'
        mouse_down = True
        user.mouse_event(2, 0, 0, 0, 0)
        user.mouse_event(4, 0, 0, 0, 0)
        mouse_down = False
        time.sleep(.25)
        guard()

    def send(vk=0, scan=0, flags=0, release=False):
        if not release:
            guard()
        event = Input(1, Payload(keyboard=Keyboard(vk, scan, flags, 0, 0)))
        assert user.SendInput(1, C.byref(event), C.sizeof(Input)) == 1, C.get_last_error()

    def chord(key):
        try:
            send(0x11)
            injected_keys.add(0x11)
            send(key)
            injected_keys.add(key)
            send(key, flags=2, release=True)
            injected_keys.discard(key)
        finally:
            if 0x11 in injected_keys:
                send(0x11, flags=2, release=True)
                injected_keys.discard(0x11)

    def type_text(text):
        for char in text:
            if char == '\n':
                send(0x0d)
                send(0x0d, flags=2, release=True)
            else:
                units = char.encode('utf-16-le')
                for offset in range(0, len(units), 2):
                    scan = int.from_bytes(units[offset:offset + 2], 'little')
                    send(scan=scan, flags=4)
                    send(scan=scan, flags=6, release=True)

    def wait_state(predicate, description, timeout=8):
        deadline = time.monotonic() + timeout
        while True:
            state = cli('inspect')
            if predicate(state):
                return state
            assert time.monotonic() < deadline, (description, state)
            time.sleep(.15)

    def fold_button(state):
        scale = user.GetDpiForWindow(hwnd) / 96
        right = state['position']['x'] + state['size']['width']
        return right - (9 if state['folded'] else 135) * scale, state['position']['y'] + (6 if state['folded'] else 15) * scale

    try:
        try:
            first = cli('open', '--title', 'Windows 실제 편집 검증', '--file', str(source))
        finally:
            found = processes()
            assert len(found) == 1, ('새 개발 PID 식별 실패', found)
            owned_pid = found.pop()
        title = f"Agent Gibbon [{first['dev_label']}] — {note_id}"
        hwnd = user.FindWindowW(None, title)
        identity()
        # Finish the cursor-directed entrance without clicking another app.
        assert user.GetForegroundWindow() == original_foreground, 'open이 foreground를 변경했습니다.'
        monitor = MonitorInfo(size=C.sizeof(MonitorInfo))
        assert user.GetMonitorInfoW(user.MonitorFromWindow(hwnd, 2), C.byref(monitor))
        direction = -40 if original_cursor.x + 40 >= monitor.work.right else 40
        assert user.SetCursorPos(original_cursor.x + direction, original_cursor.y)
        wait_state(lambda value: not value['animating'], '등장 애니메이션 완료')
        state = cli('inspect')
        scale = user.GetDpiForWindow(hwnd) / 96
        # CSS controls are 19 DIP wide with a 1 DIP gap; edit is fifth from right.
        click(state['position']['x'] + state['size']['width'] - 95 * scale,
              state['position']['y'] + 15 * scale, activate_from=original_foreground)
        chord(0x41)  # Ctrl+A, editor textarea is focused by its own click handler.
        type_text(replacement)
        chord(0x53)  # Ctrl+S, invoke the real UI save path.
        saved = wait_state(lambda value: value['note']['markdown'] == replacement, 'Ctrl+S 본문 저장')
        assert source.read_text(encoding='utf-8') == replacement, '연결 원본 저장 실패'
        assert canonical(saved['note']['source_path']) == canonical(source)
        print('PASS: 연필 편집, Unicode 입력, Ctrl+S, 메모 및 연결 원본 저장', flush=True)
        time.sleep(.8)  # Let the post-edit content-fit request complete before dragging.
        state = cli('inspect')
        assert user.GetMonitorInfoW(user.MonitorFromWindow(hwnd, 2), C.byref(monitor))
        p, size = state['position'], state['size']
        x, y = p['x'] + size['width'] - 2 * scale, p['y'] + size['height'] - 2 * scale
        dx = 60 * scale if monitor.work.right - (p['x'] + size['width']) > 70 * scale else -45 * scale
        dy = 45 * scale if monitor.work.bottom - (p['y'] + size['height']) > 55 * scale else -30 * scale
        assert size['width'] + dx >= 150 * scale and size['height'] + dy >= 60 * scale, '리사이즈 공간 부족'
        hit(x, y)
        mouse_down = True
        user.mouse_event(2, 0, 0, 0, 0)
        try:
            time.sleep(.15)
            for step in range(1, 9):
                move(x + dx * step / 8, y + dy * step / 8)
                time.sleep(.04)
        finally:
            user.mouse_event(4, 0, 0, 0, 0)
            mouse_down = False
        resized = wait_state(lambda value: value['size'] != size, '실제 가장자리 리사이즈')
        preserved_size = resized['size']
        print('RESIZE:', json.dumps({'before': size, 'after': preserved_size}), flush=True)
        time.sleep(.6)
        assert cli('inspect')['size'] == preserved_size, '수동 크기가 자동 맞춤으로 변경됨'
        cli('update', '--file', str(source), '--status', '수동 크기 보존')
        # Revision refresh schedules fit asynchronously; an immediate inspect
        # alone would miss a native drag that failed to mark the size manual.
        time.sleep(.8)
        after_update = cli('inspect')
        assert after_update['size'] == preserved_size, ('update가 수동 크기를 변경함', preserved_size, after_update['size'])
        cli('close')
        after_hide_foreground = user.GetForegroundWindow()
        assert cli('inspect')['size'] == preserved_size, 'close가 수동 크기를 변경함'
        cli('open')
        # Hide cancels any new entrance animation before comparing size.
        cli('close')
        assert cli('inspect')['size'] == preserved_size, 'close/open이 수동 크기를 변경함'
        cli('open')
        user.SetCursorPos(original_cursor.x, original_cursor.y)
        time.sleep(.1)
        user.SetCursorPos(original_cursor.x + direction, original_cursor.y)
        state = wait_state(lambda value: not value['animating'], '다시 열기 애니메이션 완료')
        click(*fold_button(state), activate_from=after_hide_foreground)
        folded = wait_state(lambda value: value['folded'], '접기 버튼')
        assert folded['size']['width'] == preserved_size['width']
        click(*fold_button(folded))
        unfolded = wait_state(lambda value: not value['folded'], '펼치기 버튼')
        assert unfolded['size'] == preserved_size, 'fold/unfold가 수동 크기를 변경함'
        assert unfolded['note']['markdown'] == replacement
        print(json.dumps({'ok': True, 'checks': ['real editor Ctrl+A/Ctrl+S', 'linked source save',
            'native edge drag', 'manual size survives update/close/open/fold/unfold'],
            'instance': first['instance'], 'size': preserved_size}, ensure_ascii=False))
    finally:
        for key in list(injected_keys):
            send(key, flags=2, release=True)
        if mouse_down:
            user.mouse_event(4, 0, 0, 0, 0)
        user.SetCursorPos(original_cursor.x, original_cursor.y)
        if hwnd and user.GetForegroundWindow() == hwnd:
            user.SetForegroundWindow(original_foreground)
        # Never let a failed close or a failed assertion skip process cleanup.
        if owned_pid is not None:
            current = processes()
            assert not current or current == {owned_pid}, '종료 전 정확한 실행 파일/PID 재확인 실패'
            if owned_pid in current:
                os.kill(owned_pid, 15)
            deadline = time.monotonic() + 8
            while owned_pid in processes():
                assert time.monotonic() < deadline, '자체 개발 프로세스 종료 대기 실패'
                time.sleep(.1)
        # Remove explicit files and empty directories only, after the app stops.
        if storage is not None:
            storage.unlink(missing_ok=True)
            storage.parent.rmdir()
        source.unlink(missing_ok=True)
        temporary.rmdir()


if __name__ == '__main__':
    main()
