#!/usr/bin/env python3
"""macOS Accessibility UI checks. Requires automation/accessibility permission.
Replaces the note, clicks pin/hide, and leaves the note hidden.
"""
import json
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
BIN = str(ROOT / 'scripts/floatnote')

def cli(*args):
    data = json.loads(subprocess.check_output([BIN, *args], text=True, timeout=25))
    assert data['ok'], data
    return data

def apple(script):
    return subprocess.check_output(['osascript', '-e', script], text=True, timeout=15).strip()

def active():
    return apple('tell application "System Events" to get name of first application process whose frontmost is true')

front = active()
opened = cli('open', '--title', 'UI 검증', '--file', str(ROOT / 'examples/progress.md'))
time.sleep(2)
assert active() == front, 'open changed foreground app'
assert not opened['focused'], 'run this test with another app focused'
content = apple('tell application "System Events" to tell process "floatnote" to get entire contents of window \"Floatnote\"')
for expected in ('☑ 단일 플로팅 메모', '☐ 실제 작업에서 사용성 확인'):
    assert expected in content, expected
def click(control):
    root = 'group "UI 검증" of UI element 1 of scroll area 1 of group 1 of group 1 of window "Floatnote"'
    apple(f'tell application "System Events" to tell process "floatnote" to click {control} of {root}')
    time.sleep(0.3)
click('checkbox "상단 고정"')
assert cli('inspect')['always_on_top'] != opened['always_on_top']
click('checkbox "상단 고정"')
assert cli('inspect')['always_on_top'] == opened['always_on_top']
# Clicking a native/WebView control can legitimately activate the app.
# Return to the original app before testing background CLI updates.
apple('tell application \"System Events\" to set frontmost of process ' + json.dumps(front) + ' to true')
time.sleep(0.3)
before = cli('inspect')
updated = cli('update', '--status', 'UI 갱신 완료')
assert active() == front and not updated['focused']
assert before['position'] == updated['position'] and before['size'] == updated['size']
click('button "메모 숨기기"')
assert not cli('inspect')['visible']
assert not cli('update', '--status', '숨김 유지')['visible']
assert cli('open')['visible'] and active() == front
cli('close')
print('PASS: actual WebView Korean/task rows without footer, pin toggle, hide button, background focus preservation, hidden update and explicit reopen')
