#!/usr/bin/env python3
"""macOS Accessibility checks of per-window WebViews and native buttons.
Creates two isolated notes, restores the foreground app, and hides test notes.
"""
import json
from pathlib import Path
import subprocess
import time
import tempfile
import uuid

ROOT = Path(__file__).resolve().parents[1]
BIN = str(ROOT / 'scripts/floatnote')


def cli(*args):
    result = json.loads(subprocess.check_output([BIN, *args], text=True, timeout=25))
    assert result['ok'], result
    return result


def apple(script):
    return subprocess.check_output(['osascript', '-e', script], text=True, timeout=15).strip()


def active():
    return apple('tell application "System Events" to get name of first application process whose frontmost is true')


def ref(note_id):
    return 'window ' + json.dumps('Floatnote — ' + note_id, ensure_ascii=False)


def contents(note_id):
    return apple('tell application "System Events" to tell process "floatnote" to get entire contents of ' + ref(note_id))


def click(note_id, control):
    title = cli('inspect', '--id', note_id)['note']['title'] or '메모 내용'
    root = 'group ' + json.dumps(title, ensure_ascii=False) + ' of UI element 1 of scroll area 1 of group 1 of group 1 of ' + ref(note_id)
    apple(f'tell application "System Events" to tell process "floatnote" to click {control} of {root}')
    time.sleep(0.3)


def restore_front(name):
    apple('tell application "System Events" to set frontmost of process ' + json.dumps(name, ensure_ascii=False) + ' to true')
    time.sleep(0.3)


front = active()
prefix = 'ui-' + uuid.uuid4().hex[:8]
a, b = prefix + '-a', prefix + '-b'
created = []
try:
    first = cli('open', '--id', a, '--title', '노란 메모', '--color', '#fff1b8', '--markdown', '# 첫째 창\n- [x] 알파 완료')
    created.append(a)
    second = cli('open', '--id', b, '--title', '파란 메모', '--color', '#18304a', '--markdown', '# 둘째 창\n- [ ] 베타 대기')
    created.append(b)
    time.sleep(2)
    assert active() == front, 'new windows changed foreground app'
    for note_id, present, absent in [(a, '☑ 알파 완료', '베타 대기'), (b, '☐ 베타 대기', '알파 완료')]:
        text = contents(note_id)
        assert present in text and absent not in text, text
        assert '갱신' not in text
    # Drag from rendered text, then empty body space; no tiny handle or modifier.
    frame = apple('tell application "System Events" to tell process "floatnote" to get {position, size} of ' + ref(a))
    x, y, width, height = map(float, frame.split(','))
    before_drag = cli('inspect', '--id', a)
    subprocess.run(['/usr/bin/swift', str(ROOT / 'scripts/macos-input.swift'), 'drag', str(x + 60), str(y + 33), str(x + 100), str(y + 53)], check=True, capture_output=True, timeout=15)
    time.sleep(.4)
    after_drag = cli('inspect', '--id', a)
    assert after_drag['position'] != before_drag['position'], 'dragging rendered text did not move the window'
    assert cli('inspect', '--id', b)['position'] == second['position']
    frame = apple('tell application "System Events" to tell process "floatnote" to get {position, size} of ' + ref(a))
    x, y, width, height = map(float, frame.split(','))
    subprocess.run(['/usr/bin/swift', str(ROOT / 'scripts/macos-input.swift'), 'drag', str(x + width / 2), str(y + height - 30), str(x + width / 2 + 20), str(y + height - 10)], check=True, capture_output=True, timeout=15)
    time.sleep(.4)
    after_blank_drag = cli('inspect', '--id', a)
    assert after_blank_drag['position'] != after_drag['position'], 'dragging empty body space did not move the window'

    frame = apple('tell application "System Events" to tell process "floatnote" to get {position, size} of ' + ref(a))
    x, y, width, height = map(float, frame.split(','))
    subprocess.run(['/usr/bin/swift', str(ROOT / 'scripts/macos-input.swift'), 'drag', str(x + width - 2), str(y + height - 2), str(x + width + 28), str(y + height + 18)], check=True, capture_output=True, timeout=15)
    time.sleep(.4)
    after_resize = cli('inspect', '--id', a)
    assert after_resize['size']['width'] > after_drag['size']['width']
    assert after_resize['size']['height'] > after_drag['size']['height']
    assert cli('inspect', '--id', b)['size'] == second['size']
    with tempfile.TemporaryDirectory(prefix='floatnote-clipboard-') as folder:
        saved = str(Path(folder) / 'clipboard.plist')
        helper = str(ROOT / 'scripts/macos-clipboard.swift')
        subprocess.run(['/usr/bin/swift', helper, 'save', saved], check=True, capture_output=True, timeout=15)
        copied = False
        try:
            before_copy = cli('inspect', '--id', a)
            click(a, 'button "본문 복사"')
            pasted = subprocess.check_output(['/usr/bin/pbpaste'], text=True)
            copied = '알파 완료' in pasted
            assert copied and '첫째 창' in pasted and '베타 대기' not in pasted
            assert '본문 복사' not in pasted and '메모 숨기기' not in pasted
            after_copy = cli('inspect', '--id', a)
            assert after_copy['position'] == before_copy['position'], 'copy button dragged the window'
            assert after_copy['size'] == before_copy['size']
        finally:
            # Do not replace a new clipboard value copied concurrently by the user.
            current = subprocess.check_output(['/usr/bin/pbpaste'], text=True)
            if copied and current == pasted:
                subprocess.run(['/usr/bin/swift', helper, 'restore', saved], check=True, capture_output=True, timeout=15)
    click(a, 'checkbox "상단 고정"')
    assert cli('inspect', '--id', a)['always_on_top'] != first['always_on_top']
    assert cli('inspect', '--id', b)['always_on_top'] == second['always_on_top']
    click(a, 'checkbox "상단 고정"')
    restore_front(front)
    before = cli('inspect', '--id', b)
    cli('update', '--id', b, '--markdown', '둘째 창만 변경', '--color', '#dcecff')
    time.sleep(0.8)
    assert active() == front
    assert '둘째 창만 변경' in contents(b)
    assert '알파 완료' in contents(a)
    after = cli('inspect', '--id', b)
    for field in ('position', 'size', 'focused', 'visible', 'always_on_top'):
        assert before[field] == after[field], field
    click(a, 'button "메모 숨기기"')
    assert not cli('inspect', '--id', a)['visible']
    assert cli('inspect', '--id', b)['visible']
    assert not cli('update', '--id', a, '--color', '#112233')['visible']
    restore_front(front)
    assert cli('open', '--id', a)['visible'] and active() == front
    print('PASS: separate WebView contents, Korean/task rows without footer, per-window pin/hide controls, content refresh, whole-body text/blank drag, copy button and clipboard restoration, corner resize, background focus preservation, hidden color update and explicit reopen')
finally:
    for note_id in created:
        cli('close', '--id', note_id)
    restore_front(front)
