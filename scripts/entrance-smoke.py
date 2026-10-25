#!/usr/bin/env python3
"""Cursor entrance acceptance test. Requires an isolated app binary/socket.
FLOATNOTE_TEST_BIN must point to a separately isolated QA instance, never the user app.
FLOATNOTE_POINTER_BIN is the compiled entrance-pointer.swift helper.
Only unique notes are created; the pointer and frontmost application are restored.
"""
import json
import os
import subprocess
import time
import uuid

BIN = os.environ['FLOATNOTE_TEST_BIN']
POINTER = os.environ['FLOATNOTE_POINTER_BIN']
assert 'qa7' in BIN, 'Use a separately isolated qa7 binary'

def pointer(*args):
    return subprocess.check_output([POINTER, *map(str, args)], text=True)

def cli(action, note_id, *args):
    result = subprocess.run([BIN, action, '--id', note_id, *args], text=True,
                            capture_output=True, timeout=12)
    data = json.loads(result.stdout)
    assert result.returncode == 0 and data['ok'], (result.stderr, data)
    return data

def wait_done(note_id):
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        state = cli('inspect', note_id)
        if not state['animating']:
            return state
        time.sleep(.04)
    raise AssertionError('entrance did not complete after pointer movement')

def same_position(a, b):
    return all(abs(a['position'][k] - b['position'][k]) <= 2 for k in ('x', 'y'))

original = json.loads(pointer())
front = subprocess.check_output(['osascript', '-e',
    'tell application "System Events" to get name of first application process whose frontmost is true'], text=True).strip()
ids = ['entrance-' + uuid.uuid4().hex[:10] for _ in range(2)]
try:
    x, y = original['width'] - 60, original['height'] - 70
    pointer('move', x, y)
    started = time.monotonic()
    state = cli('open', ids[0], '--markdown', '# 커서 등장 검증\n- [ ] 대기')
    elapsed = time.monotonic() - started
    assert elapsed < 2.5, ('CLI waited for mouse/animation', elapsed)
    assert state['visible'] and state['animating'] and state['opacity'] == 1.0, state
    scale = original['scale']
    right = (state['position']['x'] + state['size']['width']) / scale
    bottom = (state['position']['y'] + state['size']['height']) / scale
    assert 8 <= x - right <= 32 and 8 <= y - bottom <= 32, (state, x, y, scale)
    assert not state['focused'], 'open stole focus'
    time.sleep(3.6)
    waiting = cli('inspect', ids[0])
    assert waiting['animating'] and waiting['opacity'] == 1.0 and same_position(waiting, state), waiting
    reopened = cli('open', ids[0])
    assert same_position(reopened, state), 'visible open restarted entrance'
    cli('update', ids[0], '--markdown', '# 갱신\n' + '내용\n' * 20)
    assert same_position(cli('inspect', ids[0]), state), 'update moved waiting note'
    pointer('move', x - 10, y)
    landed = wait_done(ids[0])
    assert landed['opacity'] == .5 and not landed['focused'], landed
    assert not same_position(landed, state), 'note did not travel to destination'
    time.sleep(.8)
    fitted = cli('inspect', ids[0])
    assert fitted['size']['height'] > state['size']['height'], 'deferred auto sizing lost'
    print('PASS: nearby opaque wait >3s, bounded CLI, visible-open/update preservation, movement and auto sizing', flush=True)
    cli('close', ids[0])
    pointer('move', 60, 100)
    restored = cli('open', ids[0])
    assert restored['animating'] and restored['opacity'] == 1.0, restored
    assert restored['position']['x'] / scale >= 68, restored
    closed = cli('close', ids[0])
    assert not closed['visible'] and not closed['animating'], closed
    cli('update', ids[0], '--status', 'hidden')
    pointer('move', 80, 100)
    time.sleep(.6)
    assert not cli('inspect', ids[0])['visible'], 'hidden note reappeared'
    second = cli('open', ids[1], '--markdown', '독립 노트')
    assert second['visible'] and second['animating'], second
    assert not cli('inspect', ids[0])['visible']
    pointer('move', 90, 100)
    wait_done(ids[1])
    print('PASS: hidden restore, cancellation, hidden update, independent notes', flush=True)
finally:
    for note_id in ids:
        subprocess.run([BIN, 'close', '--id', note_id], capture_output=True, timeout=12)
    pointer('move', original['x'], original['y'])
    subprocess.run(['osascript', '-e', 'tell application "System Events" to set frontmost of process ' + json.dumps(front) + ' to true'], capture_output=True)
