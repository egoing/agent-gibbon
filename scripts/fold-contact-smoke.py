#!/usr/bin/env python3
"""Verify short Ctrl contacts in this worktree's isolated development app.
Requires macOS event-posting permission; restores pointer and hides the unique note.
Run after npm run dev:build, without touching the mouse/keyboard during the test.
"""
import json
from pathlib import Path
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
CLI = str(ROOT / 'scripts/agent-gibbon-dev')
BINARY = str(ROOT / 'src-tauri/target/debug/agent-gibbon')

def pids():
    lines = subprocess.check_output(['ps', '-axo', 'pid=,comm='], text=True).splitlines()
    return {int(line.strip().split(None, 1)[0]) for line in lines
            if len(line.strip().split(None, 1)) == 2 and line.strip().split(None, 1)[1] == BINARY}

before_pids = pids()
note = 'fold-contact-' + uuid.uuid4().hex[:8]

def cli(action, *args):
    result = subprocess.run([CLI, action, '--id', note, *args], capture_output=True, text=True, timeout=25)
    assert result.returncode == 0, result.stdout + result.stderr
    data = json.loads(result.stdout)
    assert data['ok'], data
    return data

with tempfile.TemporaryDirectory(prefix='gibbon-fold-') as temp:
    source = Path(temp) / 'input.swift'
    helper = Path(temp) / 'input'
    source.write_text(r'''
import AppKit
import CoreGraphics
let a = CommandLine.arguments
func move(_ x:Double, _ y:Double, _ control:Bool) {
 let p = CGPoint(x:x, y:y)
 CGWarpMouseCursorPosition(p)
 let e = CGEvent(mouseEventSource:nil, mouseType:.mouseMoved, mouseCursorPosition:p, mouseButton:.left)!
 e.flags = control ? .maskControl : []
 e.post(tap:.cghidEventTap)
}
switch a[1] {
case "access": print(CGPreflightPostEventAccess())
case "pointer":
 let p = CGEvent(source:nil)!.location
 print("\(p.x) \(p.y)")
case "modifiers": print(NSEvent.modifierFlags.rawValue & NSEvent.ModifierFlags.deviceIndependentFlagsMask.rawValue)
case "move": move(Double(a[2])!, Double(a[3])!, a[4] == "ctrl")
case "touch":
 let x = Double(a[2])!, y = Double(a[3])!
 move(x, y, true)
 usleep(20_000)
 move(x, Double(a[4])!, true)
case "windows":
 let all = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as! [[String:Any]]
 let matches = all.filter { ($0[kCGWindowOwnerPID as String] as? Int) == Int(a[2])! }
 print(String(data:try! JSONSerialization.data(withJSONObject:matches.map { $0[kCGWindowBounds as String]! }), encoding:.utf8)!)
default: fatalError()
}
''')
    subprocess.run(['swiftc', str(source), '-o', str(helper)], check=True, capture_output=True)
    def native(*args):
        return subprocess.check_output([str(helper), *map(str, args)], text=True, timeout=10).strip()
    assert native('access') == 'true', 'Event posting permission unavailable'
    assert int(native('modifiers')) == 0, 'Release modifier keys before running'
    original = native('pointer').split()
    created = False
    try:
        cli('open', '--markdown', '# 접촉 검증\n본문 유지', '--title', '접촉 검증')
        created = True
        time.sleep(1.1)
        native('move', 50, 50, 'none')
        time.sleep(.8)
        initial = cli('inspect')
        pid, = pids()
        bounds = [b for b in json.loads(native('windows', pid)) if b['Width'] >= 144]
        assert len(bounds) == 1, 'Run with other development notes hidden'
        b = bounds[0]
        x, y, outside = b['X'] + 30, b['Y'] + 6, b['Y'] - 30
        native('move', x, outside, 'none')
        subprocess.run(['osascript', '-e', 'tell application "System Events" to key down control'], check=True)
        time.sleep(.1)
        native('move', x, y, 'ctrl')
        time.sleep(.2)
        assert cli('inspect')['folded'], ('Initial contact did not fold', b, native('pointer'), native('modifiers'), cli('inspect'))
        native('move', x, outside, 'ctrl')
        time.sleep(.1)
        for i in range(10):
            native('touch', x, y, outside)
            time.sleep(.12)
            state = cli('inspect')
            assert state['folded'] == (i % 2 == 1), ('short contact missed', i, state)
        print('PASS: 10 repeated 20ms contacts, including unfolding the 12px title', flush=True)
        # The note is folded. Contact 3px above the visible title exercises its margin.
        native('touch', x, b['Y'] - 3, outside)
        time.sleep(.15)
        assert not cli('inspect')['folded'], 'Folded contact margin failed'
        # The same location must not fold an expanded note.
        native('touch', x, b['Y'] - 3, outside)
        time.sleep(.15)
        final = cli('inspect')
        assert not final['folded'], 'Margin leaked to expanded note'
        for key in ('position', 'size', 'focused', 'always_on_top', 'note'):
            assert final[key] == initial[key], (key, initial[key], final[key])
        print('PASS: folded-only margin; size, position, focus, pin and body preserved', flush=True)
        cli('close')
        native('touch', x, y, outside)
        time.sleep(.15)
        assert not cli('inspect')['visible'], 'Hidden note reopened'
        print('PASS: hidden note stays hidden', flush=True)
    finally:
        subprocess.run(['osascript', '-e', 'tell application "System Events" to key up control'], check=True)
        native('move', *original, 'none')
        if created:
            cli('close')
        # Stop only the exact worktree binary started by this test.
        import os
        import signal
        for pid in pids() - before_pids:
            os.kill(pid, signal.SIGTERM)
