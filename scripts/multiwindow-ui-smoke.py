#!/usr/bin/env python3
"""macOS Accessibility checks of per-window WebViews and native buttons.
Creates two isolated notes, restores the foreground app, and hides test notes.
"""
import json
from pathlib import Path
import subprocess
import time
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
    root = 'group 1 of UI element 1 of scroll area 1 of group 1 of group 1 of ' + ref(note_id)
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
        assert '갱신' in text
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
    print('PASS: separate WebView contents, Korean/tasks/timestamp, per-window pin/hide controls, content refresh, background focus preservation, hidden color update and explicit reopen')
finally:
    for note_id in created:
        cli('close', '--id', note_id)
    restore_front(front)
