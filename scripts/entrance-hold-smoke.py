#!/usr/bin/env python3
"""Verify the one-second minimum hold in an isolated qa7 app."""
import json
import os
import subprocess
import time
import uuid

BIN = os.environ['AGENT_GIBBON_TEST_BIN']
POINTER = os.environ['AGENT_GIBBON_POINTER_BIN']
assert 'qa7' in BIN

def cli(action, note, *args):
    result = json.loads(subprocess.check_output([BIN, action, '--id', note, *args], text=True, timeout=12))
    assert result['ok'], result
    return result

def move(x, y):
    subprocess.run([POINTER, 'move', str(x), str(y)], check=True)

original = json.loads(subprocess.check_output([POINTER], text=True))
note = 'hold-' + uuid.uuid4().hex[:10]
try:
    move(original['width'] - 60, original['height'] - 70)
    start = cli('open', note, '--markdown', '# 최소 1초 대기 검증')
    shown = time.monotonic()
    assert start['visible'] and start['animating'], start
    move(original['width'] - 80, original['height'] - 70)
    # Inspect throughout the guard, rather than only before the pointer moves.
    samples = 0
    while time.monotonic() - shown < .65:
        current = cli('inspect', note)
        assert current['position'] == start['position'], current
        assert current['opacity'] == 1.0 and current['animating'], current
        samples += 1
        time.sleep(.03)
    assert samples >= 3, samples
    time.sleep(.9)
    landed = cli('inspect', note)
    assert not landed['animating'] and landed['position'] != start['position'], landed
    # The random destination may be under the pointer; test outside opacity explicitly.
    move(0, 0)
    assert cli('inspect', note)['opacity'] == .5, landed
    print(f'PASS: {samples} stationary opaque samples after early pointer movement; flight completes after minimum hold')
finally:
    subprocess.run([BIN, 'close', '--id', note], capture_output=True)
    move(original['x'], original['y'])
