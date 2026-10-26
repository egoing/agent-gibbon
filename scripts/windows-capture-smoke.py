"""Windows real capture/text-selection regression; run only in an assigned GUI slot.

python scripts/windows-capture-smoke.py --interact
The primary screen is covered by this test's opaque native pattern fixture before
capture. Only a verified rectangle inside that fixture is selected for storage.
No screenshots of the desktop are written by this script. No clipboard access.
"""
import argparse
import ctypes as C
from ctypes import wintypes as W
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import sys
import time
import uuid

from windows_uia import control_center
from windows_capture_pattern import add_pattern, calibrate_palette, expected_color, verify_png

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'src-tauri/target/debug/agent-gibbon.exe'
CLI = ROOT / 'scripts/agent-gibbon-dev.cmd'


class Keyboard(C.Structure):
    _fields_ = [('vk', W.WORD), ('scan', W.WORD), ('flags', W.DWORD),
                ('time', W.DWORD), ('extra', C.c_size_t)]


class Mouse(C.Structure):
    _fields_ = [('dx', W.LONG), ('dy', W.LONG), ('data', W.DWORD),
                ('flags', W.DWORD), ('time', W.DWORD), ('extra', C.c_size_t)]


class Payload(C.Union):
    _fields_ = [('keyboard', Keyboard), ('mouse', Mouse)]


class Input(C.Structure):
    _fields_ = [('kind', W.DWORD), ('payload', Payload)]


def native():
    user = C.WinDLL('user32', use_last_error=True)
    user.SetProcessDpiAwarenessContext.argtypes = [C.c_void_p]
    user.SetProcessDpiAwarenessContext(C.c_void_p(-4))
    user.GetForegroundWindow.restype = W.HWND
    user.SetForegroundWindow.argtypes = [W.HWND]
    user.GetWindowThreadProcessId.argtypes = [W.HWND, C.POINTER(W.DWORD)]
    user.GetWindowTextW.argtypes = [W.HWND, W.LPWSTR, C.c_int]
    user.FindWindowW.argtypes = [W.LPCWSTR, W.LPCWSTR]
    user.FindWindowW.restype = W.HWND
    user.IsWindowVisible.argtypes = [W.HWND]
    user.GetCursorPos.argtypes = [C.POINTER(W.POINT)]
    user.SetCursorPos.argtypes = [C.c_int, C.c_int]
    user.GetClientRect.argtypes = [W.HWND, C.POINTER(W.RECT)]
    user.GetWindowRect.argtypes = [W.HWND, C.POINTER(W.RECT)]
    user.GetWindowLongW.argtypes = [W.HWND, C.c_int]
    user.ClientToScreen.argtypes = [W.HWND, C.POINTER(W.POINT)]
    user.GetDpiForWindow.argtypes = [W.HWND]
    user.GetDpiForWindow.restype = W.UINT
    user.WindowFromPoint.argtypes = [W.POINT]
    user.WindowFromPoint.restype = W.HWND
    user.GetAncestor.argtypes = [W.HWND, W.UINT]
    user.GetAncestor.restype = W.HWND
    user.GetAsyncKeyState.argtypes = [C.c_int]
    user.GetAsyncKeyState.restype = C.c_short
    user.RegisterHotKey.argtypes = [W.HWND, C.c_int, W.UINT, W.UINT]
    user.UnregisterHotKey.argtypes = [W.HWND, C.c_int]
    user.SendInput.argtypes = [W.UINT, C.POINTER(Input), C.c_int]
    user.SendInput.restype = W.UINT
    user.mouse_event.argtypes = [W.DWORD, W.DWORD, W.DWORD, W.DWORD, C.c_size_t]
    return user


def fixture(token):
    """Separate process owns its message loop and a full-primary white STATIC."""
    user = native()
    user.CreateWindowExW.argtypes = [W.DWORD, W.LPCWSTR, W.LPCWSTR, W.DWORD,
        C.c_int, C.c_int, C.c_int, C.c_int, W.HWND, W.HMENU, W.HINSTANCE, C.c_void_p]
    user.CreateWindowExW.restype = W.HWND
    # SS_WHITERECT + WS_POPUP + WS_VISIBLE; no text or private content is drawn.
    hwnd = user.CreateWindowExW(8, 'STATIC', 'Gibbon capture fixture ' + token,
        0x90000006, 0, 0, user.GetSystemMetrics(0), user.GetSystemMetrics(1), None, None, None, None)
    assert hwnd, C.get_last_error()
    add_pattern(user, hwnd, user.GetSystemMetrics(0), user.GetSystemMetrics(1))
    message = W.MSG()
    while user.GetMessageW(C.byref(message), None, 0, 0) > 0:
        user.TranslateMessage(C.byref(message))
        user.DispatchMessageW(C.byref(message))


def processes():
    result = subprocess.run(['powershell', '-NoProfile', '-Command',
        "@(Get-CimInstance Win32_Process -Filter \"Name = 'agent-gibbon.exe'\" | "
        "Where-Object { $_.ExecutablePath -eq $env:GIBBON_CAPTURE_BINARY } | "
        'ForEach-Object { $_.ProcessId }) | ConvertTo-Json -Compress'],
        env={**os.environ, 'GIBBON_CAPTURE_BINARY': str(BINARY)}, capture_output=True,
        text=True, timeout=20, check=True)
    value = json.loads(result.stdout) if result.stdout.strip() else []
    return set(value if isinstance(value, list) else [value])


def canonical(path):
    return os.path.normcase(os.path.realpath(path)).removeprefix('\\\\?\\')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--interact', action='store_true')
    modes.add_argument('--fixture', metavar='UUID', help=argparse.SUPPRESS)
    args = parser.parse_args()
    assert sys.platform == 'win32', 'Windows 전용 검증입니다.'
    if args.fixture:
        assert re.fullmatch('[0-9a-f]{32}', args.fixture)
        return fixture(args.fixture)
    user = native()
    assert BINARY.is_file(), '먼저 개발 앱을 빌드하세요.'
    assert not processes(), '해당 워크트리 개발 앱이 이미 실행 중입니다.'
    watched = (0x10, 0x11, 0x12, 0x5b, 0x5c, 1, 2, 4)
    assert not any(user.GetAsyncKeyState(key) < 0 for key in watched), '사용자 입력 중: 중단'
    # Probe before starting our app. Never send a chord already owned by another
    # app, nor unregister a hotkey registered by a different process.
    registered = []
    try:
        for key in (0x31, 0x32):
            hotkey_id = 0x6300 + key
            assert user.RegisterHotKey(None, hotkey_id, 6, key), (
                '다른 앱이 Ctrl+Shift 캡처 단축키를 점유 중입니다. 입력 없이 중단합니다.', key)
            registered.append(hotkey_id)
    finally:
        for hotkey_id in registered:
            user.UnregisterHotKey(None, hotkey_id)
    original_foreground = user.GetForegroundWindow()
    original_cursor = W.POINT()
    assert user.GetCursorPos(C.byref(original_cursor))
    token = uuid.uuid4().hex
    seed = 'capture-smoke-' + token
    fixture_title = 'Gibbon capture fixture ' + token
    child = None
    fixture_hwnd = None
    owned_pid = None
    baseline = None
    document_root = None
    original_directories = None
    selections = []
    documents = {}
    held = set()
    button_down = False
    primary_rect = None

    def pid_of(hwnd):
        value = W.DWORD()
        user.GetWindowThreadProcessId(hwnd, C.byref(value))
        return value.value

    def title_of(hwnd):
        text = C.create_unicode_buffer(512)
        user.GetWindowTextW(hwnd, text, len(text))
        return text.value

    def identity(hwnd):
        assert hwnd and user.IsWindowVisible(hwnd), '대상 창이 보이지 않습니다.'
        pid = pid_of(hwnd)
        if child and pid == child.pid:
            assert hwnd == fixture_hwnd and title_of(hwnd) == fixture_title
        else:
            assert owned_pid and pid == owned_pid, '다른 프로세스로 입력하지 않습니다.'

    def guard(hwnd, foreground=True):
        identity(hwnd)
        if foreground:
            assert user.GetForegroundWindow() == hwnd, 'foreground 변경: 입력 중단'
        assert not any(user.GetAsyncKeyState(key) < 0 for key in watched
                       if key not in held and not (key == 1 and button_down)), '사용자 입력 감지'

    def send(hwnd, vk=0, scan=0, flags=0, release=False):
        if not release:
            guard(hwnd)
        event = Input(1, Payload(keyboard=Keyboard(vk, scan, flags, 0, 0)))
        assert user.SendInput(1, C.byref(event), C.sizeof(Input)) == 1, C.get_last_error()

    def chord(hwnd, modifiers, key):
        try:
            for modifier in modifiers:
                send(hwnd, modifier)
                held.add(modifier)
            # Preserve a realistic Ctrl lead-in when testing the text editor.
            if modifiers == [0x11]:
                time.sleep(.15)
            send(hwnd, key)
            held.add(key)
        finally:
            for released in [key, *reversed(modifiers)]:
                if released in held:
                    send(hwnd, released, flags=2, release=True)
                    held.remove(released)

    def move_hit(hwnd, x, y, foreground=True):
        guard(hwnd, foreground)
        assert user.SetCursorPos(round(x), round(y))
        assert user.GetAncestor(user.WindowFromPoint(W.POINT(round(x), round(y))), 2) == hwnd, '잘못된 마우스 대상'

    def click(hwnd, x, y, foreground=True):
        nonlocal button_down
        move_hit(hwnd, x, y, foreground)
        button_down = True
        user.mouse_event(2, 0, 0, 0, 0)
        user.mouse_event(4, 0, 0, 0, 0)
        button_down = False
        time.sleep(.25)
        guard(hwnd)

    def client(hwnd):
        rect, origin = W.RECT(), W.POINT()
        assert user.GetClientRect(hwnd, C.byref(rect))
        assert user.ClientToScreen(hwnd, C.byref(origin))
        return origin.x, origin.y, rect.right, rect.bottom

    def geometry(stage, state):
        hwnd = user.FindWindowW(None, f"Agent Gibbon [{state['dev_label']}] — {state['id']}")
        assert hwnd and pid_of(hwnd) == owned_pid
        outer = W.RECT()
        assert user.GetWindowRect(hwnd, C.byref(outer))
        print('IMAGE_GEOMETRY', json.dumps({'stage': stage, 'client': client(hwnd),
            'outer': [outer.left, outer.top, outer.right - outer.left, outer.bottom - outer.top],
            'style': hex(user.GetWindowLongW(hwnd, -16) & 0xffffffff),
            'extended': hex(user.GetWindowLongW(hwnd, -20) & 0xffffffff),
            'capture': state['note']['capture']}), flush=True)

    def cli(action, *values):
        result = subprocess.run(['cmd.exe', '/d', '/c', str(CLI), action, *values], cwd=ROOT,
                                capture_output=True, encoding='utf-8', timeout=35)
        assert result.returncode == 0, (result.stdout, result.stderr)
        value = json.loads(result.stdout)
        assert value['ok'], value
        if 'storage_path' in value:
            note_id = value['id']
            assert note_id == seed or (baseline is not None and note_id not in baseline
                and re.fullmatch('(image|text)-[0-9]+', note_id)), '소유하지 않은 메모'
            identifier = json.loads((ROOT / 'src-tauri/tauri.conf.json').read_text(encoding='utf-8'))['identifier']
            expected = Path(os.environ['APPDATA']) / identifier / 'notes' / value['instance'] / 'docs' / note_id / 'index.md'
            assert canonical(value['storage_path']) == canonical(expected), '예상 밖 저장 위치'
            documents[note_id] = expected
        return value

    def ids():
        return {item['id'] for item in cli('list')['windows']}

    def wait_for(check, description, timeout=15):
        deadline = time.monotonic() + timeout
        while True:
            value = check()
            if value:
                return value
            assert time.monotonic() < deadline, description
            time.sleep(.1)

    def note_window(state):
        hwnd = user.FindWindowW(None, f"Agent Gibbon [{state['dev_label']}] — {state['id']}")
        identity(hwnd)
        return hwnd

    def fixture_ready():
        # Hide every generated note before each snapshot: the entire selected
        # monitor then contains only the opaque, nonsensitive fixture window.
        for note_id in documents:
            cli('close', '--id', note_id)
        x, y, width, height = primary_rect
        identity(fixture_hwnd)
        click(fixture_hwnd, x + width / 2, y + height / 2, foreground=False)
        calibrate_palette(user, fixture_hwnd, (width, height))
        # Read only verified visible pixels of the opaque native fixture.
        gdi = C.WinDLL('gdi32')
        user.GetDC.argtypes = [W.HWND]
        user.GetDC.restype = W.HDC
        user.ReleaseDC.argtypes = [W.HWND, W.HDC]
        gdi.GetPixel.argtypes = [W.HDC, C.c_int, C.c_int]
        gdi.GetPixel.restype = W.DWORD
        dc = user.GetDC(None)
        try:
            for px, py in [(width // 2, height // 2), (width // 2 - 220, height // 2 - 130),
                           (width // 2 + 220, height // 2 + 130)]:
                assert user.GetAncestor(user.WindowFromPoint(W.POINT(x + px, y + py)), 2) == fixture_hwnd
                actual, expected = gdi.GetPixel(dc, x + px, y + py), expected_color(px, py, (width, height))
                assert actual == expected, ('fixture 패턴 불일치: 캡처 중단', px, py, hex(actual), hex(expected))
        finally:
            user.ReleaseDC(None, dc)

    def overlay(mode):
        fixture_ready()
        chord(fixture_hwnd, [0x11, 0x10], 0x31 if mode == 'image' else 0x32)
        expected = '영역 캡처 — Escape로 취소' if mode == 'image' else '빈 노트 영역 선택 — Escape로 취소'
        def find():
            x, y, width, height = primary_rect
            hwnd = user.GetAncestor(user.WindowFromPoint(W.POINT(x + width // 2, y + height // 2)), 2)
            if pid_of(hwnd) == owned_pid and title_of(hwnd) == expected and user.IsWindowVisible(hwnd):
                return hwnd
            foreground = user.GetForegroundWindow()
            assert pid_of(foreground) in (owned_pid, child.pid), '다른 앱이 단축키를 처리했습니다. 재시도하지 않습니다.'
        hwnd = wait_for(find, '자기 앱 캡처 overlay가 없습니다. 단축키 충돌/캡처 오류를 확인하세요.')
        wait_for(lambda: client(hwnd) == primary_rect, 'overlay client 영역이 모니터 physical 영역과 다릅니다.')
        outer = W.RECT()
        assert user.GetWindowRect(hwnd, C.byref(outer))
        print('OVERLAY_GEOMETRY', json.dumps({'mode': mode, 'client': client(hwnd),
            'monitor': primary_rect, 'outer': [outer.left, outer.top,
                outer.right - outer.left, outer.bottom - outer.top]}), flush=True)
        # Multi-monitor capture may focus another overlay of this same PID.
        guard(hwnd, foreground=False)
        return hwnd

    def select(mode):
        nonlocal button_down
        before = ids()
        hwnd = overlay(mode)
        x, y, width, height = primary_rect
        start = (x + width // 2 - 220, y + height // 2 - 130)
        end = (start[0] + 440, start[1] + 260)
        assert x + 32 < start[0] < end[0] < x + width - 32
        assert y + 32 < start[1] < end[1] < y + height - 32
        # Every pointer sample stays inside the frozen fixture's verified bounds.
        move_hit(hwnd, *start, foreground=False)
        button_down = True
        user.mouse_event(2, 0, 0, 0, 0)
        try:
            time.sleep(.08)
            for step in range(1, 9):
                move_hit(hwnd, start[0] + 440 * step / 8, start[1] + 260 * step / 8)
                time.sleep(.025)
        finally:
            selections.append((mode, time.time_ns()))
            user.mouse_event(4, 0, 0, 0, 0)
            button_down = False
        created = wait_for(lambda: ids() - before, '영역 선택 메모 생성 실패')
        assert len(created) == 1 and next(iter(created)).startswith(mode + '-'), created
        return cli('inspect', '--id', created.pop())

    def ratio_drag(hwnd, expected_ratio):
        nonlocal button_down
        def chrome_ready():
            try:
                control_center(hwnd, '메모 접기')
                return True
            except AssertionError:
                return False
        # The backend unfolds before its IPC response updates the WebView's
        # folded class (which hides all resize grips). Wait for own UIA chrome.
        wait_for(chrome_ready, '이미지 펼침 UI 갱신 대기')
        x, y, width, height = client(hwnd)
        assert abs(width / height - expected_ratio) < .025, ('초기 이미지 비율 오류', width, height)
        click(hwnd, x + width / 2, y + height / 2, foreground=False)
        scale = user.GetDpiForWindow(hwnd) / 96
        start = (x + width - scale, y + height - scale)
        move_hit(hwnd, *start)
        button_down = True
        user.mouse_event(2, 0, 0, 0, 0)
        try:
            time.sleep(.15)
            for step in range(1, 9):
                # Deliberately request unequal deltas. Native sizing owns mouse
                # capture, so its constraint may move the border off the pointer.
                guard(hwnd)
                assert user.SetCursorPos(round(start[0] - 80 * scale * step / 8),
                                         round(start[1] - 15 * scale * step / 8))
                time.sleep(.04)
        finally:
            user.mouse_event(4, 0, 0, 0, 0)
            button_down = False
        time.sleep(.3)
        _, _, resized_width, resized_height = client(hwnd)
        print('IMAGE_RATIO_DRAG', json.dumps({'before': [width, height],
            'after': [resized_width, resized_height], 'expected_ratio': expected_ratio}), flush=True)
        assert (resized_width, resized_height) != (width, height), '이미지 리사이즈 입력이 적용되지 않았습니다.'
        assert abs(resized_width / resized_height - expected_ratio) < .025, (
            '이미지 비율 유지 실패', (width, height), (resized_width, resized_height))
        return resized_width, resized_height

    try:
        try:
            bootstrap = cli('open', '--id', seed, '--markdown', '# Capture regression fixture')
        finally:
            found = processes()
            assert len(found) == 1, ('자기 개발 프로세스 식별 실패', found)
            owned_pid = found.pop()
        baseline = ids()
        document_root = documents[seed].parent.parent
        original_directories = {path.name for path in document_root.iterdir()}
        child = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), '--fixture', token], cwd=ROOT)
        fixture_hwnd = wait_for(lambda: user.FindWindowW(None, fixture_title), 'fixture 생성 실패')
        primary_rect = client(fixture_hwnd)
        assert primary_rect[2] >= 640 and primary_rect[3] >= 480, 'fixture 선택 공간 부족'
        initial_ids = ids()
        cancelled = overlay('image')
        # Escape is sent only to our foreground capture overlay, never another app.
        foreground = user.GetForegroundWindow()
        assert pid_of(foreground) == owned_pid and 'Escape로 취소' in title_of(foreground)
        send(foreground, 0x1b)
        send(foreground, 0x1b, flags=2, release=True)
        wait_for(lambda: not user.IsWindowVisible(cancelled), 'Escape 취소 실패')
        assert user.GetForegroundWindow() == fixture_hwnd, 'Escape 취소가 기존 fixture 포커스를 복원하지 않았습니다.'
        assert ids() == initial_ids, 'Escape가 메모를 생성했습니다.'
        print('PASS: Ctrl+Shift+1 overlay와 Escape 취소', flush=True)

        image = select('image')
        assert user.GetForegroundWindow() == fixture_hwnd, '이미지 메모 생성이 기존 fixture 포커스를 변경했습니다.'
        geometry('created', image)
        time.sleep(.5)
        geometry('created-settled', image)
        assert image['note']['capture'] and not image['note']['markdown']
        png = documents[image['id']].with_name('capture.png').read_bytes()
        assert png[:8] == b'\x89PNG\r\n\x1a\n'
        width, height = struct.unpack('>II', png[16:24])
        assert abs(width - 440) <= 2 and abs(height - 260) <= 2, ('fixture crop 크기 오류', width, height)
        x, y, fw, fh = primary_rect
        selected_origin = (x + fw // 2 - 220, y + fh // 2 - 130)
        bounds = image['note']['capture']
        assert abs(bounds['x'] - selected_origin[0]) <= .01 and abs(bounds['y'] - selected_origin[1]) <= .01, ('저장된 선택 원점 불일치', bounds, selected_origin)
        assert client(note_window(image)) == (*selected_origin, 440, 260), '이미지 client 영역과 실제 선택 영역 불일치'
        verify = verify_png(documents[image['id']].with_name('capture.png'),
            selection_origin=selected_origin, fixture_origin=(x, y), fixture_size=(fw, fh))
        print('PASS: 선택 원점/배율/색상/알파 원본 패턴', json.dumps(verify), flush=True)

        text = select('text')
        geometry('text-created', text)
        assert text['note']['edit_on_open'] and text['note']['markdown'] == '' and not text['note']['capture']
        text_hwnd = note_window(text)
        assert client(text_hwnd) == (*selected_origin, 440, 260), '빈 텍스트 client 영역과 실제 선택 영역 불일치'
        click(text_hwnd, *control_center(text_hwnd, 'Markdown 원문'))
        chord(text_hwnd, [0x11], 0x41)
        body = '영역 텍스트 메모 저장 확인'
        encoded = body.encode('utf-16-le')
        for offset in range(0, len(encoded), 2):
            scan = int.from_bytes(encoded[offset:offset + 2], 'little')
            send(text_hwnd, scan=scan, flags=4)
            send(text_hwnd, scan=scan, flags=6, release=True)
        chord(text_hwnd, [0x11], 0x53)
        wait_for(lambda: cli('inspect', '--id', text['id'])['note']['markdown'] == body, '영역 텍스트 Ctrl+S 저장 실패')
        click(text_hwnd, *control_center(text_hwnd, '메모 접기'))
        wait_for(lambda: cli('inspect', '--id', text['id'])['folded'], '메모 접기 실패')
        x, y, width, height = client(text_hwnd)
        click(text_hwnd, x + width * .25, y + height / 2)
        wait_for(lambda: not cli('inspect', '--id', text['id'])['folded'], '접힌 메모 본문 클릭으로 펼치기 실패')
        print('PASS: Ctrl+Shift+2 빈 편집기/저장/접힌 메모 전체 클릭', flush=True)

        cli('close', '--id', text['id'])
        geometry('hidden-before-reopen', image)
        cli('open', '--id', image['id'])
        geometry('reopened', image)
        image_hwnd = note_window(image)
        expected_ratio = image['note']['capture']['width'] / image['note']['capture']['height']
        resized = ratio_drag(image_hwnd, expected_ratio)
        click(image_hwnd, *control_center(image_hwnd, '메모 접기'))
        wait_for(lambda: cli('inspect', '--id', image['id'])['folded'], '이미지 접기 실패')
        x, y, width, height = client(image_hwnd)
        click(image_hwnd, x + width * .25, y + height / 2)
        wait_for(lambda: not cli('inspect', '--id', image['id'])['folded'], '이미지 전체 클릭 펼치기 실패')
        assert all(abs(actual - expected) <= 1 for actual, expected in zip(client(image_hwnd)[2:], resized)), '이미지 접기/펼치기 크기 복원 실패'
        ratio_drag(image_hwnd, expected_ratio)
        print(json.dumps({'ok': True, 'checks': ['capture shortcut', 'Escape cancellation',
            'fixture-only crop', 'text shortcut auto-editor and Ctrl+S', 'folded whole-note click',
            'image aspect ratio during real edge drag before/after fold']}, ensure_ascii=False))
    finally:
        cleanup_errors = []
        original_error = sys.exc_info()[1]
        for key in list(held):
            try:
                send(None, key, flags=2, release=True)
            except Exception as error:
                cleanup_errors.append(error)
        if button_down:
            user.mouse_event(4, 0, 0, 0, 0)
        foreground_pid = pid_of(user.GetForegroundWindow())
        user.SetCursorPos(original_cursor.x, original_cursor.y)
        if foreground_pid in {owned_pid, child.pid if child else None}:
            user.SetForegroundWindow(original_foreground)
        if owned_pid is not None:
            try:
                current = processes()
                assert not current or current == {owned_pid}, '정확한 바이너리/PID 재확인 실패'
                # Do not send another CLI request to a deadlocked app. The
                # filesystem baseline below also covers notes saved before
                # WebView construction could add them to the live note list.
                if owned_pid in current:
                    os.kill(owned_pid, 15)
                wait_for(lambda: owned_pid not in processes(), '자체 개발 앱 종료 실패', timeout=10)
            except Exception as error:
                cleanup_errors.append(error)
        if child is not None:
            try:
                if child.poll() is None:
                    child.terminate()
                child.wait(timeout=10)
            except Exception as error:
                cleanup_errors.append(error)
        try:
            assert not processes(), '앱이 살아 있으므로 저장 파일을 삭제하지 않습니다.'
            if document_root is not None and original_directories is not None:
                finished_ns = time.time_ns()
                for path in document_root.iterdir():
                    if path.name in original_directories:
                        continue
                    match = re.fullmatch('(image|text)-([0-9]+)', path.name)
                    if not match or not path.is_dir() or path.is_symlink():
                        continue
                    mode, stamp = match.group(1), int(match.group(2))
                    if not any(selected_mode == mode and started <= stamp <= finished_ns
                               for selected_mode, started in selections):
                        continue
                    expected = document_root / path.name / 'index.md'
                    allowed = {'index.md', 'capture.png'} if mode == 'image' else {'index.md'}
                    assert {entry.name for entry in path.iterdir()} <= allowed, '신규 메모에 예상 밖 파일이 있어 삭제하지 않습니다.'
                    assert all(not entry.is_symlink() for entry in path.iterdir()), '저장 파일 symlink: 삭제 중단'
                    documents[path.name] = expected
            for note_id, path in documents.items():
                assert path.parent.name == note_id and path.name == 'index.md'
                if note_id.startswith('image-'):
                    path.with_name('capture.png').unlink(missing_ok=True)
                path.unlink(missing_ok=True)
                path.parent.rmdir()
        except Exception as error:
            cleanup_errors.append(error)
        for error in cleanup_errors:
            print('Cleanup:', str(error), file=sys.stderr)
        if cleanup_errors and original_error is None:
            raise RuntimeError('캡처 smoke 정리 실패') from cleanup_errors[0]


if __name__ == '__main__':
    main()
