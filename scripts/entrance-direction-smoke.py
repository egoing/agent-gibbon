#!/usr/bin/env python3
"""Exercise entrance directions in this worktree's dev app (macOS).
Run after npm run dev:build; do not move the pointer during the test.
Restores the pointer, hides test notes, and stops only a newly started dev process.
"""
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
CLI = str(ROOT / 'scripts/agent-gibbon-dev')
INFO = json.loads(subprocess.check_output(['node', str(ROOT / 'scripts/dev.mjs'), 'info'], text=True))

def pids():
    rows = subprocess.check_output(['ps', '-axo', 'pid=,comm='], text=True).splitlines()
    return {int(parts[0]) for row in rows if len(parts := row.strip().split(None, 1)) == 2 and parts[1] == INFO['binary']}

before = pids()
notes = []
def cli(action, note, *args):
    value = json.loads(subprocess.check_output([CLI, action, '--id', note, *args], text=True, timeout=25))
    assert value['ok'], value
    return value

with tempfile.TemporaryDirectory(prefix='gibbon-direction-') as temp:
    helper = str(Path(temp) / 'pointer')
    subprocess.run(['swiftc', str(ROOT / 'scripts/entrance-pointer.swift'), '-o', helper], check=True)
    input_helper = str(Path(temp) / 'input')
    subprocess.run(['swiftc', str(ROOT / 'scripts/macos-input.swift'), '-o', input_helper], check=True)
    original = json.loads(subprocess.check_output([helper], text=True))
    scale = original['scale']
    cx, cy = original['width'] / 2, original['height'] / 2
    def move(x, y):
        subprocess.run([helper, 'move', str(x), str(y)], check=True)
    def landed(note):
        time.sleep(1.6)
        result = cli('inspect', note)
        assert not result['animating'], result
        return result
    def frame(value):
        return [value['position']['x'] / scale, value['position']['y'] / scale, value['size']['width'] / scale, value['size']['height'] / scale]
    def native(action, *coords):
        move(coords[0], coords[1])
        time.sleep(.05)
        subprocess.run([input_helper, action, *map(str, coords)], check=True)
    try:
        for direction, dx, dy in [('right', 24, 0), ('left', -24, 0), ('up', 0, -24), ('down', 0, 24)]:
            note = 'direction-' + uuid.uuid4().hex[:10]
            notes.append(note)
            move(cx, cy)
            start = cli('open', note, '--markdown', '# 방향 검증\n\n본문 보존')
            assert start['visible'] and start['animating'] and not start['focused'], start
            # Native window geometry can settle just after the open response.
            time.sleep(.1)
            start = cli('inspect', note)
            move(cx + dx, cy + dy)
            early = cli('inspect', note)
            assert early['position'] == start['position'] and early['animating'], (start, early)
            # Reversal in the hold must not cancel the pending movement.
            move(cx - dx, cy - dy)
            end = landed(note)
            assert not end['focused'] and end['note']['markdown'] == start['note']['markdown'], end
            assert end['visible'] and not end['folded'], end
            assert end['size'] == start['size'], end
            x = end['position']['x'] / scale
            assert abs(x + end['size']['width'] / scale - (original['width'] - 8)) <= 2, end
            move(cx, cy + 80)
            stable = cli('open', note)
            assert not stable['animating'] and stable['position'] == end['position'], stable
            cli('close', note)
            cli('update', note, '--markdown', '숨김 갱신')
            assert not cli('inspect', note)['visible']
            move(cx, cy)
            reopened = cli('open', note)
            assert reopened['visible'] and reopened['animating'], reopened
            # Hold longer than one second with only jitter: no automatic flight.
            move(cx + 2, cy + 2)
            time.sleep(1.15)
            waiting = cli('inspect', note)
            assert waiting['animating'] and waiting['position'] == reopened['position'], waiting
            move(cx, cy - 24 if direction == 'up' else cy)
            if direction != 'up':
                move(cx + 24, cy)
            restored = landed(note)
            assert restored['visible'] and not restored['folded'], restored
            cli('close', note)
            print('PASS:', direction, 'hold/reversal/action/reopen/jitter/one-shot', flush=True)
        note = 'direction-drag-' + uuid.uuid4().hex[:10]
        notes.append(note)
        move(cx, cy)
        start = cli('open', note, '--markdown', '# 직접 조작' + '\n\n본문' * 6)
        time.sleep(.1)
        start = cli('inspect', note)
        x, y, w, h = frame(start)
        native('drag', x + 40, y + 50, x + 100, y + 80)
        time.sleep(.6)
        dragged = cli('inspect', note)
        ax, ay, _, _ = frame(dragged)
        assert not dragged['animating'] and abs(ax - x - 60) < 8 and abs(ay - y - 30) < 8, dragged
        time.sleep(.7)
        assert cli('inspect', note)['position'] == dragged['position'], 'drag cancellation resumed'
        x, y, w, h = frame(dragged)
        native('drag', x + w - 5, y + h - 5, x + w + 35, y + h + 25)
        time.sleep(.3)
        resized = cli('inspect', note)
        assert resized['size']['width'] > dragged['size']['width'], resized
        cli('close', note)
        move(cx, cy)
        cli('open', note)
        move(cx, cy - 24)
        restored = landed(note)
        assert not restored['folded'] and restored['size'] == resized['size'], (resized, restored)
        cli('close', note)
        print('PASS: native drag cancellation; manual size survives movement', flush=True)
        # Concurrent entrances must reserve distinct empty destinations.
        occupied = []
        move(cx, cy)
        for _ in range(3):
            note = 'space-' + uuid.uuid4().hex[:10]
            notes.append(note)
            cli('open', note, '--markdown', '# 빈 공간 검증')
            occupied.append(note)
        move(cx - 24, cy)
        rectangles = [frame(landed(note)) for note in occupied]
        for i, (x, y, w, h) in enumerate(rectangles):
            for ox, oy, ow, oh in rectangles[:i]:
                assert x + w <= ox or ox + ow <= x or y + h <= oy or oy + oh <= y, rectangles
        print('PASS: concurrent entrances reserve non-overlapping destinations', flush=True)
    finally:
        for note in notes:
            subprocess.run([CLI, 'close', '--id', note], capture_output=True)
        move(original['x'], original['y'])
        for pid in pids() - before:
            # Recheck exact executable before terminating only this test's process.
            if pid in pids():
                os.kill(pid, signal.SIGTERM)
