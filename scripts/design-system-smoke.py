#!/usr/bin/env python3
"""macOS native Feedback regression. Uses only this worktree's development CLI.
Requires the existing System Events automation/accessibility permission.
"""
import json
from pathlib import Path
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
BIN = str(ROOT / 'scripts/agent-gibbon-dev')
ID = 'feedback-smoke-' + uuid.uuid4().hex[:8]

def cli(action, *args):
    data = json.loads(subprocess.check_output([BIN, action, '--id', ID, *args], text=True))
    assert data['ok'], data
    return data

def wait_for(predicate):
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        state = cli('inspect')
        if predicate(state):
            return state
        time.sleep(.15)
    raise AssertionError(state)

def apple(script):
    return subprocess.check_output(['osascript', '-e', script], text=True, stderr=subprocess.PIPE, timeout=20).strip()

def ui(action, *args, feedback=False):
    title = f"Agent Gibbon [{label}] — {ID}" + (' · Feedback' if feedback else '')
    command = [str(helper), str(pid), title, action, *args]
    for attempt in range(30):
        result = subprocess.run(command, text=True, capture_output=True)
        if result.returncode == 0:
            return result.stdout
        if attempt == 29:
            raise RuntimeError(result.stderr)
        time.sleep(.1)

def click(description, feedback=False):
    ui('click', description, feedback=feedback)

def draft(text):
    assert ui('draft', text).rstrip('\n') == text

import tempfile
helper_folder = tempfile.TemporaryDirectory(prefix='gibbon-feedback-ax-')
helper = Path(helper_folder.name) / 'feedback-ax'
subprocess.run(['swiftc', str(ROOT / 'scripts/design-system-ax.swift'), '-o', str(helper)], check=True)

front = apple('tell application "System Events" to get unix id of first application process whose frontmost is true')
created = False
try:
    state = cli('open', '--title', 'Feedback 검증', '--color', '#fff5cf', '--markdown', '# 원본\n본문')
    created = True
    label = state['dev_label']
    expected = str(ROOT / 'src-tauri/target/debug/agent-gibbon') + ' --serve'
    processes = subprocess.check_output(['ps', '-axo', 'pid=,command='], text=True).splitlines()
    matches = [int(line.strip().split(None, 1)[0]) for line in processes if line.strip().split(None, 1)[-1] == expected]
    assert len(matches) == 1, matches
    pid = matches[0]
    cli('update', '--markdown', '# 변경\n본문')
    state = wait_for(lambda s: s['feedback_surface'] and s['feedback_surface']['visible'] and s['feedback_surface']['size']['height'] == 36 * s['size']['width'] / 300 and s['feedback_surface']['position']['x'] + s['feedback_surface']['size']['width'] == s['position']['x'] + s['size']['width'])
    surface = state['feedback_surface']
    scale = state['size']['width'] / 300
    assert surface['opacity'] == state['opacity'], state
    assert surface['position']['y'] == state['position']['y'] + state['size']['height'] or surface['position']['y'] + surface['size']['height'] == state['position']['y'], state
    assert not surface['focused'] and not state['focused'], state
    assert surface['size']['width'] <= state['size']['width'], state
    assert surface['size']['height'] == 36 * state['size']['width'] / 300, surface
    assert surface['position']['x'] + surface['size']['width'] == state['position']['x'] + state['size']['width']
    assert 'AXScrollBar' not in ui('dump', feedback=True)
    click('확인', True)
    wait_for(lambda s: s['highlight_revision'] is None and not s['feedback_surface']['visible'])
    print('PASS: external right alignment, compact height, no activation, explicit acknowledgement', flush=True)
    click('메모 편집')
    wait_for(lambda s: [b['id'] for b in s['feedback_surface']['view']['buttons']] == ['cancel', 'save'])
    draft('# 저장 성공\n한글 내용')
    click('저장', True)
    state = wait_for(lambda s: s['note']['markdown'] == '# 저장 성공\n한글 내용' and s['feedback_surface']['view']['message'] == '저장했습니다')
    assert not state['feedback_surface']['view']['buttons']
    wait_for(lambda s: not s['feedback_surface']['visible'])
    print('PASS: Action Bar saves draft, becomes Toast, then disappears', flush=True)
    click('메모 편집')
    draft('보존할 초안')
    ui('resize', '144', '160')
    cli('update', '--markdown', '외부에서 수정한 내용')
    click('저장', True)
    state = wait_for(lambda s: s['feedback_surface']['view']['error'] and s['feedback_surface']['size']['width'] <= s['size']['width'] and s['feedback_surface']['size']['height'] > 32 * scale)
    assert [b['id'] for b in state['feedback_surface']['view']['buttons']] == ['cancel', 'save']
    assert state['note']['markdown'] == '외부에서 수정한 내용'
    assert '보존할 초안' in ui('dump')
    assert state['feedback_surface']['size']['width'] <= state['size']['width']
    assert state['feedback_surface']['size']['height'] > 32 * scale
    click('상단 고정')
    state = wait_for(lambda s: not s['always_on_top'] and not s['feedback_surface']['always_on_top'])
    click('상단 고정')
    wait_for(lambda s: s['always_on_top'] and s['feedback_surface']['always_on_top'])
    print('PASS: 144px memo wraps error, preserves draft/actions and synchronizes pin', flush=True)
    click('취소', True)
    wait_for(lambda s: [b['id'] for b in s['feedback_surface']['view']['buttons']] == ['acknowledge'])
    click('메모 접기')
    wait_for(lambda s: s['folded'] and not s['feedback_surface']['visible'])
    click('메모 펼치기')
    wait_for(lambda s: not s['folded'] and s['feedback_surface']['visible'])
    cli('close')
    wait_for(lambda s: not s['visible'] and not s['feedback_surface']['visible'])
    cli('update', '--markdown', '숨긴 상태의 갱신')
    state = wait_for(lambda s: not s['visible'] and not s['feedback_surface']['visible'])
    print('PASS: fold/unfold and hidden updates preserve companion visibility', flush=True)
finally:
    if created:
        cli('close')
    apple('tell application "System Events" to set frontmost of (first application process whose unix id is ' + front + ') to true')

helper_folder.cleanup()
