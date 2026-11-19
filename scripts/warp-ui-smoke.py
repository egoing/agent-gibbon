#!/usr/bin/env python3
"""macOS warp smoke test. Use an isolated development app and an unlocked desktop.
Requires JAJAN_TEST_BIN and JAJAN_TEST_PROCESS.
Compiles a temporary Swift cursor helper; requires macOS accessibility access.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
BIN = os.environ.get('JAJAN_TEST_BIN', str(ROOT / 'scripts/jajan'))
PROCESS = os.environ.get('JAJAN_TEST_PROCESS', 'jajan')

def apple(code):
    return subprocess.check_output(['osascript', '-e', code], text=True, timeout=15).strip()

def ui(code):
    return apple(f'tell application "System Events" to tell process {json.dumps(PROCESS)} to {code}')

def cli(action, note_id, *args):
    data = json.loads(subprocess.check_output([BIN, action, '--id', note_id, *args], text=True, timeout=25))
    assert data['ok'], data
    return data

def passed(message):
    print('PASS: ' + message, flush=True)


assert os.environ.get('JAJAN_TEST_BIN') and os.environ.get('JAJAN_TEST_PROCESS'), 'Use an isolated app'
pointer_folder = tempfile.TemporaryDirectory(prefix='jajan-warp-pointer-')
pointer_source = Path(pointer_folder.name) / 'pointer.swift'
pointer_source.write_text("""import CoreGraphics
import Foundation
let point = CGPoint(x: Double(CommandLine.arguments[1])!, y: Double(CommandLine.arguments[2])!)
CGWarpMouseCursorPosition(point)
""")
POINTER = str(Path(pointer_folder.name) / 'pointer')
subprocess.run(['swiftc', str(pointer_source), '-o', POINTER], check=True)
front = apple('tell application "System Events" to get name of first application process whose frontmost is true')
note_id = 'warp-test-' + uuid.uuid4().hex[:8]
window = 'window ' + json.dumps('Jajan — ' + note_id, ensure_ascii=False)
def inspect(): return cli('inspect', note_id)
def move(x, y):
    subprocess.run([POINTER, str(x), str(y)], check=True)
    time.sleep(.5)
def position():
    return [int(x.strip()) for x in ui('get position of ' + window).split(',')]
def contact():
    x, y = position()
    move(x + 30, y + 35)

try:
    initial = cli('open', note_id, '--markdown', '# 워프 검증')
    time.sleep(.8)
    x, y = position()
    # Leave the source first; native warp requires a fresh pointer entry.
    move(max(0, x - 20) if x >= 20 else x + initial['size']['width'] + 20, y + 35)
    apple('tell application "System Events" to set frontmost of process ' + json.dumps(front, ensure_ascii=False) + ' to true')
    apple('tell application "System Events" to key down option')
    contact()
    left = inspect()
    assert left['position']['x'] != initial['position']['x'], (initial, left)
    assert left['position']['y'] == initial['position']['y']
    assert not left['focused']
    time.sleep(.8)
    assert inspect()['position'] == left['position'], 'Repeated warp without new contact'
    for key in ('size', 'always_on_top', 'note'):
        assert left[key] == initial[key], key
    contact()
    assert inspect()['position'] == initial['position']
    passed('Option-pointer round trip, no repeat, focus and content preserved')
    cli('close', note_id)
    move(x + 30, y + 35)
    assert inspect()['position'] == initial['position']
    passed('hidden memo stays at its original position')
finally:
    apple('tell application "System Events" to key up option')
    cli('close', note_id)
    apple('tell application "System Events" to set frontmost of process ' + json.dumps(front, ensure_ascii=False) + ' to true')
