#!/usr/bin/env python3
"""Exercise the actual menu-bar icon; restore pointer/focus and hide test note."""
import json
from pathlib import Path
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
INFO = json.loads(subprocess.check_output(['node',str(ROOT/'scripts/dev.mjs'),'info'],text=True))


def cli(*args):
    d = json.loads(subprocess.check_output([str(ROOT / 'scripts/agent-gibbon-dev'), *args], text=True, timeout=25))
    assert d['ok'], d
    return d


def apple(script):
    return subprocess.check_output(['osascript', '-e', script], text=True, timeout=15).strip()


def choose(label):
    frame = apple(f'tell application "System Events" to tell (first application process whose unix id is {pid}) to get {{position, size}} of menu bar item 1 of menu bar 2')
    x, y, w, h = map(float, frame.split(','))
    subprocess.run(['/usr/bin/swift', str(ROOT / 'scripts/macos-input.swift'), 'click', str(x + w / 2), str(y + h / 2)], check=True, capture_output=True, timeout=15)
    # Background inspection/status updates must not replace and dismiss the open menu.
    cli('inspect', '--id', note_id)
    cli('update', '--id', note_id, '--status', '메뉴 열림 중 갱신')
    apple(f'tell application "System Events" to tell (first application process whose unix id is {pid}) to click menu item ' + json.dumps(label, ensure_ascii=False) + ' of menu 1 of menu bar item 1 of menu bar 2')
    time.sleep(.5)


front = apple('tell application "System Events" to get name of first application process whose frontmost is true')
note_id = 'tray-' + uuid.uuid4().hex[:8]
cli('open', '--id', note_id, '--title', '복원 검증', '--markdown', '메뉴 막대에서 다시 여는 메모')
state=cli('inspect','--id',note_id)
assert state['instance']==INFO['id'] and state['id']==note_id
rows=[r.strip().split(None,1) for r in subprocess.check_output(['ps','-axo','pid=,comm='],text=True).splitlines()]
pid=next(int(p) for p,path in rows if path==INFO['binary'])
try:
    before = cli('close', '--id', note_id)
    choose('복원 검증')
    after = cli('inspect', '--id', note_id)
    assert after['visible'], after
    for field in ('position', 'size', 'always_on_top'):
        assert before[field] == after[field], field
    print('PASS: actual menu-bar icon, menu survives background inspect/update, individual restore preserves position/size/pin')
finally:
    cli('close', '--id', note_id)
    apple('tell application "System Events" to set frontmost of process ' + json.dumps(front, ensure_ascii=False) + ' to true')
