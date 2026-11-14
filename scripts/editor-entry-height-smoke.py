#!/usr/bin/env python3
"""PID-bound macOS editor height regression, this worktree's dev instance only."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
OUT = Path(os.environ.get('GIBBON_SMOKE_OUTPUT', '/tmp/gibbon159-native'))
OUT.mkdir(parents=True, exist_ok=True)
info = json.loads(subprocess.check_output(['node', 'scripts/dev.mjs', 'info'], cwd=ROOT, text=True))
assert info['root'] == str(ROOT)
ID = 'editor-height-' + uuid.uuid4().hex[:8]
MARKER = '# 편집 높이 검증 ' + ID
records = []
pid = None
created = False

def run(*args):
    return subprocess.check_output(list(map(str, args)), text=True, timeout=30).strip()

def cli(action, *args):
    value = json.loads(run(ROOT / 'scripts/agent-gibbon-dev', action, '--id', ID, *args))
    assert value['ok'], value
    if action == 'inspect':
        assert value['instance'] == info['id'] and value['id'] == ID, value
    return value

def wait(predicate):
    deadline = time.monotonic() + 7
    while time.monotonic() < deadline:
        value = cli('inspect')
        if predicate(value):
            return value
        time.sleep(.1)
    raise AssertionError(value)

def ax(action, *args, feedback=False):
    return run(OUT / 'ax', pid, title, action, *args)

def geometry():
    return json.loads(ax('geometry'))

def record(name):
    state = cli('inspect')
    records.append({'case': name, 'inspect': state, 'ax': geometry()})
    return state

def click(name, feedback=False):
    ax('click', name, feedback=feedback)
    time.sleep(.25)

def editing():
    return wait(lambda s: s['feedback_surface'] and s['feedback_surface']['visible'] and
                any(b['id'] == 'save' for b in s['feedback_surface']['view']['buttons']))

def cancel():
    click('취소', feedback=True)

front = int(run('swift', ROOT / 'scripts/frontmost.swift'))
try:
    for source, name in [('design-system-ax.swift', 'ax'), ('auto-layout-native.swift', 'native')]:
        subprocess.run(['swiftc', str(ROOT / 'scripts' / source), '-o', str(OUT / name)], check=True)
    with tempfile.TemporaryDirectory(prefix='gibbon159-source-') as folder:
        source = Path(folder) / 'source.md'
        source.write_text(MARKER)
        opened = cli('open', '--file', source, '--title', '편집 높이 검증')
        created = True
        expected = info['binary'] + ' --serve'
        matches = [int(line.strip().split(None, 1)[0]) for line in run('ps', '-axo', 'pid=,command=').splitlines()
                   if line.strip().split(None, 1)[-1] == expected]
        assert len(matches) == 1, matches
        pid = matches[0]
        title = f"Agent Gibbon [{opened['dev_label']}] — {ID}"
        state = wait(lambda s: s['visible'] and s['note']['markdown'] == MARKER)
        native = json.loads(run(OUT / 'native', pid))
        screens = native['screens']
        primary_top = screens[0]['frame'][1] + screens[0]['frame'][3]
        work = screens[0]['visible']
        x, y = work[0] + 80, primary_top - work[1] - work[3] + 80
        # A real position change cancels the unrelated cursor-wait animation.
        ax('position', x, y)
        wait(lambda s: not s['animating'])
        ax('resize', 300, 48)
        time.sleep(.3)
        scale = cli('inspect')['size']['width'] / 300
        for width, height in [(144, 32), (300, 48), (300, 209), (300, 210), (300, 280)]:
            ax('position', x, y)
            ax('resize', width, height)
            time.sleep(.3)
            before = record(f'{width}x{height}-before')
            click('메모 편집')
            state = editing()
            assert state['size'] == {'width': round(width * scale), 'height': round(max(210, height) * scale)}, state
            assert state['position'] == before['position'], (before, state)
            assert state['always_on_top'] == before['always_on_top'] and state['focused'] == before['focused']
            box = geometry()['editors'][0]['rect']
            assert box['height'] > 100 and box['width'] > 100, box
            assert geometry()['editors'][0]['value'] == MARKER
            record(f'{width}x{height}-edit')
            cancel()
            assert cli('inspect')['size'] == state['size']
        # Entering at the bottom grows upward; Feedback stays inside the memo.
        ax('position', x, primary_top - work[1] - 48)
        ax('resize', 300, 48)
        time.sleep(.4)
        before = record('bottom-before')
        click('메모 편집')
        state = editing()
        assert state['size']['height'] == round(210 * scale)
        assert state['position']['x'] == before['position']['x']
        assert state['position']['y'] < before['position']['y']
        surface = state['feedback_surface']
        assert surface['mode'] == 'inline' and surface['visible'], surface
        record('bottom-edit')
        # Capture only this PID/title window, never a user's window.
        windows = json.loads(run(OUT / 'native', pid))['windows']
        target = next(w for w in windows if w.get('kCGWindowName') == title)
        subprocess.run(['screencapture', '-x', '-l', str(target['kCGWindowNumber']), str(OUT / 'bottom-edit.png')], check=True)
        # A manual resize after entry is preserved across polling and hide/reopen.
        ax('draft', '보존할 초안')
        ax('resize', 300, 96)
        time.sleep(1)
        assert cli('inspect')['size']['height'] == round(96 * scale)
        cli('close')
        hidden = cli('inspect')
        assert not hidden['visible']
        cli('open')
        state = wait(lambda s: s['visible'])
        time.sleep(.6)
        assert geometry()['editors'][0]['value'] == '보존할 초안'
        assert cli('inspect')['size']['height'] == round(96 * scale)
        record('manual-small-hidden-restored')
        # Conflict still retains the draft; cancel and re-entry grows again.
        source.write_text('외부 원본 변경')
        click(next(b['label'] for b in cli('inspect')['feedback_surface']['view']['buttons'] if b['id'] == 'save'), feedback=True)
        state = wait(lambda s: s['feedback_surface']['view']['error'])
        assert geometry()['editors'][0]['value'] == '보존할 초안'
        assert source.read_text() == '외부 원본 변경'
        record('conflict-draft-preserved')
        cancel()
        click('메모 접기')
        wait(lambda s: s['folded'])
        click('메모 펼치기')
        wait(lambda s: not s['folded'])
        click('메모 편집')
        editing()
        assert cli('inspect')['size']['height'] == round(210 * scale)
        # Re-read the source before testing successful save via the same existing path.
        cancel()
        cli('update', '--file', source)
        time.sleep(.4)
        click('메모 편집')
        editing()
        ax('draft', '# 높이 검증 저장 완료')
        click(next(b['label'] for b in cli('inspect')['feedback_surface']['view']['buttons'] if b['id'] == 'save'), feedback=True)
        wait(lambda s: s['note']['markdown'] == '# 높이 검증 저장 완료')
        assert source.read_text() == '# 높이 검증 저장 완료'
        record('saved')
        print(json.dumps({'root': str(ROOT), 'instance': info, 'pid': pid, 'title': title,
                          'scale': scale, 'note': ID, 'cases': len(records), 'output': str(OUT)}, ensure_ascii=False))
finally:
    if created:
        cli('close')
    if front > 0:
        subprocess.run(['osascript', '-e', 'tell application "System Events" to set frontmost of (first application process whose unix id is ' + str(front) + ') to true'], check=False)
    (OUT / 'evidence.json').write_text(json.dumps({'info': info, 'pid': pid, 'title': locals().get('title'), 'records': records}, ensure_ascii=False, indent=2))
