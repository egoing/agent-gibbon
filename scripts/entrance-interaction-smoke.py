#!/usr/bin/env python3
"""Actual macOS drag/resize regression on an isolated qa7 app only."""
import json
import os
from pathlib import Path
import subprocess
import time
import uuid

BIN = os.environ['AGENT_GIBBON_TEST_BIN']
POINTER = os.environ['AGENT_GIBBON_POINTER_BIN']
INPUT = os.environ['AGENT_GIBBON_INPUT_BIN']
PROCESS = os.environ.get('AGENT_GIBBON_TEST_PROCESS', 'agent-gibbon-qa7')
assert 'qa7' in BIN and PROCESS == 'agent-gibbon-qa7'

def apple(text):
    return subprocess.check_output(['osascript', '-e', text], text=True, timeout=15).strip()

def cli(action, note, *args):
    result = json.loads(subprocess.check_output([BIN, action, '--id', note, *args], text=True, timeout=12))
    assert result['ok'], result
    return result

def move(x, y):
    subprocess.run([POINTER, 'move', str(x), str(y)], check=True)

def frame(note):
    # Native CLI geometry avoids requiring System Events AX read permission.
    # This interaction test stays on the primary display used by the pointer helper.
    state = cli('inspect', note)
    scale = original['scale']
    return [state['position']['x'] / scale, state['position']['y'] / scale,
            state['size']['width'] / scale, state['size']['height'] / scale]

def drag(x, y, dx, dy):
    subprocess.run([INPUT, 'drag', str(x), str(y), str(x + dx), str(y + dy)], check=True, timeout=10)

original = json.loads(subprocess.check_output([POINTER], text=True))
front = apple('tell application "System Events" to get name of first application process whose frontmost is true')
note = 'entrance-drag-' + uuid.uuid4().hex[:8]
try:
    move(600, 400)
    start = cli('open', note, '--markdown', '# 직접 조작\n' + '\n\n본문' * 6)
    assert start['animating']
    x, y, w, h = frame(note)
    drag(x + 60, y + 40, 80, 40)
    time.sleep(.5)
    after = cli('inspect', note)
    assert not after['animating'], after
    # Actual native drag must end at the dragged position, not at the far edge destination.
    ax, ay, _, _ = frame(note)
    assert abs(ax - (x + 80)) < 8 and abs(ay - (y + 40)) < 8, (frame(note), (x, y, w, h))
    time.sleep(.6)
    assert cli('inspect', note)['position'] == after['position'], 'cancelled flight resumed'
    print('PASS: actual drag cancels entrance and retains user position', flush=True)
    x, y, w, h = frame(note)
    drag(x + w - 2, y + h - 2, 60, 40)
    time.sleep(.5)
    resized = cli('inspect', note)
    assert resized['size']['width'] > after['size']['width'], (after, resized)
    cli('update', note, '--markdown', '짧은 본문')
    time.sleep(.8)
    assert cli('inspect', note)['size'] == resized['size'], 'manual size lost on update'
    cli('close', note)
    move(600, 400)
    reopened = cli('open', note)
    assert reopened['size'] == resized['size'] and reopened['animating'], reopened
    move(610, 400)
    time.sleep(.8)
    assert cli('inspect', note)['size'] == resized['size'], 'manual size lost after arrival'
    print('PASS: actual resize, update/hide/reopen/arrival preserve manual size', flush=True)
finally:
    subprocess.run([BIN, 'close', '--id', note], capture_output=True)
    move(original['x'], original['y'])
    apple('tell application "System Events" to set frontmost of process ' + json.dumps(front) + ' to true')
