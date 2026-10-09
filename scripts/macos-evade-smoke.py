#!/usr/bin/env python3
"""Real Option+pointer E2E. Requires macOS input automation permission.
Moves only Floatnote, restores pointer and foreground app, releases Option on exit.
Run with no modifier physically held and avoid interacting during the test.
"""
import json
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BIN = str(ROOT / 'scripts/floatnote')
def cli(*args):
    return json.loads(subprocess.check_output([BIN, *args], text=True, timeout=25))
def apple(script):
    return subprocess.check_output(['osascript', '-e', script], text=True, timeout=15).strip()
def front():
    return apple('tell application "System Events" to get name of first application process whose frontmost is true')
def unchanged(a, b):
    for key in ('size', 'focused', 'visible', 'always_on_top', 'note'):
        assert a[key] == b[key], (key, a[key], b[key])

with tempfile.TemporaryDirectory(prefix='floatnote-input-') as temp:
    driver = str(Path(temp) / 'pointer')
    subprocess.run(['swiftc', str(ROOT / 'scripts/testing/macos-pointer.swift'), '-o', driver], check=True)
    def native():
        return json.loads(subprocess.check_output([driver, 'inspect'], text=True))
    def move(x, y, option=False):
        apple('tell application \"System Events\" to key ' + ('down' if option else 'up') + ' option')
        subprocess.run([driver, 'move', str(x), str(y), 'option' if option else 'none'], check=True)
        time.sleep(0.35)
    original_front = front()
    cli('open', '--title', 'Option 회피 검증', '--markdown', '# 원본\n- [x] 유지')
    original = native()
    try:
        # A plain hover must not move the note.
        x, y = original['x'] + 30, original['y'] + 90
        move(x, y)
        before = cli('inspect')
        assert native()['x'] == original['x'], 'plain hover moved the window'
        assert front() == original_front, 'plain hover changed focus'
        # Holding Option while already inside also counts as contact.
        move(x, y, True)
        moved = native()
        assert moved['option'], 'input driver did not hold Option (permission/input error)'
        assert moved['x'] != original['x'], 'Option+pointer did not move Floatnote'
        left = original['x'] + original['width']/2 < original['screenX'] + original['screenWidth']/2
        expected = original['screenX'] + original['screenWidth'] - original['width'] - 12 if left else original['screenX'] + 12
        assert abs(moved['x'] - expected) <= 1, (moved, expected)
        assert moved['screenX'] <= moved['x'] <= moved['screenX'] + moved['screenWidth'] - moved['width']
        assert moved['screenY'] <= moved['y'] <= moved['screenY'] + moved['screenHeight'] - moved['height']
        unchanged(before, cli('inspect'))
        assert front() == original_front, 'evade changed focus'
        # No oscillation while the pointer stays at its old location.
        time.sleep(0.5)
        assert native()['x'] == moved['x'], 'window oscillated without re-entry'
        # Chase the window with Option still held: it must evade in reverse.
        move(moved['x'] + 30, moved['y'] + 90, True)
        back = native()
        assert back['x'] != moved['x'], 're-entry did not rearm'
        unchanged(before, cli('inspect'))
        # Release key: contact becomes inert again.
        move(back['x'] + 30, back['y'] + 90)
        assert native()['x'] == back['x'], 'released Option still moved the note'
        cli('close')
        hidden = cli('inspect')
        move(back['x'] + 30, back['y'] + 90, True)
        after = cli('inspect')
        assert after['position'] == hidden['position'] and not after['visible'], 'hidden note moved or appeared'
        print('PASS: plain/Option/release hover, both edges, containment, re-entry, no oscillation, focus/content/size/pin/hidden preservation')
    finally:
        apple('tell application \"System Events\" to key up option')
        move(original['pointerX'], original['pointerY'])
        apple('tell application "System Events" to set frontmost of process ' + json.dumps(original_front) + ' to true')
