#!/usr/bin/env python3
"""macOS warp smoke test. Use a fresh isolated app profile: this changes the help preference.
Requires FLOATNOTE_TEST_BIN and FLOATNOTE_TEST_PROCESS.
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
BIN = os.environ.get('FLOATNOTE_TEST_BIN', str(ROOT / 'scripts/floatnote'))
PROCESS = os.environ.get('FLOATNOTE_TEST_PROCESS', 'floatnote')

def apple(code):
    return subprocess.check_output(['osascript', '-e', code], text=True, timeout=15).strip()

def ui(code):
    return apple(f'tell application "System Events" to tell process {json.dumps(PROCESS)} to {code}')

def cli(action, note_id, *args):
    data = json.loads(subprocess.check_output([BIN, action, '--id', note_id, *args], text=True, timeout=25))
    assert data['ok'], data
    return data

def root(note_id):
    return 'group "Floatnote" of UI element 1 of scroll area 1 of group 1 of group 1 of window ' + json.dumps('Floatnote — ' + note_id, ensure_ascii=False)

def editor(note_id):
    return 'group "Markdown 편집" of ' + root(note_id)

def click(control, target):
    ui(f'click {control} of {target}')
    time.sleep(.45)

def write(note_id, text):
    ui('set focused of text area "Markdown 원문" of ' + editor(note_id) + ' to true')
    ui('set value of text area "Markdown 원문" of ' + editor(note_id) + ' to ' + json.dumps(text, ensure_ascii=False))
    assert draft(note_id) == text, 'Accessibility text input was not applied'

def draft(note_id):
    return ui('get value of text area "Markdown 원문" of ' + editor(note_id))

def contents(note_id):
    return ui('get entire contents of window ' + json.dumps('Floatnote — ' + note_id, ensure_ascii=False))

def passed(message):
    print('PASS: ' + message, flush=True)


assert os.environ.get('FLOATNOTE_TEST_BIN') and os.environ.get('FLOATNOTE_TEST_PROCESS'), 'Use an isolated app'
pointer_folder = tempfile.TemporaryDirectory(prefix='floatnote-warp-pointer-')
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
base = 'UI element 1 of scroll area 1 of group 1 of group 1 of window ' + json.dumps('Floatnote — ' + note_id, ensure_ascii=False)
tip = 'group "워프 단축키 안내" of ' + base
window = 'window ' + json.dumps('Floatnote — ' + note_id, ensure_ascii=False)
def inspect(): return cli('inspect', note_id)
def move(x, y):
    subprocess.run([POINTER, str(x), str(y)], check=True)
    time.sleep(.5)
def position():
    return [int(x.strip()) for x in ui('get position of ' + window).split(',')]
def warp(): click('button "반대편으로 워프"', root(note_id))
try:
    initial = cli('open', note_id, '--markdown', '# 워프 검증')
    time.sleep(.8)
    warp()
    left = inspect()
    assert left['position']['x'] < initial['position']['x'], (left, initial)
    assert left['position']['y'] == initial['position']['y']
    assert '워프 단축키 안내' in contents(note_id)
    click('button "워프 안내 닫기"', tip)
    assert '워프 단축키 안내' not in contents(note_id)
    warp()
    assert inspect()['position'] == initial['position']
    passed('icon round trip, initial height, shortcut hint and close')
    x, y = position()
    ui('set position of ' + window + ' to {' + str(x) + ', ' + str(y + 20) + '}')
    time.sleep(.3)
    warp()
    assert inspect()['position'] == left['position']
    passed('manual move then warp restores original vertical position')
    click('button "메모 편집"', root(note_id)); write(note_id, '워프 중 초안')
    move(700, 100)
    apple('tell application "System Events" to set frontmost of process ' + json.dumps(front, ensure_ascii=False) + ' to true')
    apple('tell application "System Events" to key down option')
    x, y = position(); move(x + 30, y + 60)
    after = inspect()
    assert after['position'] == initial['position'], after
    assert after['focused'] is False
    time.sleep(.8)
    assert inspect()['position'] == after['position'], 'Repeated warp without new contact'
    assert draft(note_id) == '워프 중 초안'
    for key in ('size', 'always_on_top', 'note'):
        assert after[key] == initial[key], key
    move(700, 100)
    x, y = position(); move(x + 30, y + 60)
    assert inspect()['position'] == left['position']
    apple('tell application "System Events" to key up option')
    passed('unfocused Alt contact, no bounce, repeat contact, draft/size/pin/content preservation')
    click('button "취소"', editor(note_id))
    warp()
    click('button "다시 보지 않기"', tip)
    warp()
    assert '워프 단축키 안내' not in contents(note_id)
    hidden_x, hidden_y = position()
    cli('close', note_id)
    hidden = inspect()
    apple('tell application "System Events" to key down option')
    # Same coordinates where the now-hidden note was last displayed.
    move(hidden_x + 30, hidden_y + 30)
    assert inspect()['position'] == hidden['position']
    apple('tell application "System Events" to key up option')
    cli('open', note_id); time.sleep(.4); warp()
    assert '워프 단축키 안내' not in contents(note_id)
    passed('persistent help dismissal and hidden notes remain unmoved')
finally:
    apple('tell application "System Events" to key up option')
    cli('close', note_id)
    apple('tell application "System Events" to set frontmost of process ' + json.dumps(front, ensure_ascii=False) + ' to true')
