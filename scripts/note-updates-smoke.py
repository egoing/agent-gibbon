#!/usr/bin/env python3
"""macOS native #65 smoke; isolated worktree CLI, test-owned process only."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import uuid

assert sys.platform == 'darwin'
ROOT = Path(__file__).resolve().parents[1]
INFO = json.loads(subprocess.check_output(['node', str(ROOT / 'scripts/dev.mjs'), 'info'], text=True))
CLI = str(ROOT / 'scripts/agent-gibbon-dev')
EXPECTED = INFO['binary'] + ' --serve'
OUTPUT = ROOT / 'docs/verification/issue65-focus'
IDS = ['updates-' + uuid.uuid4().hex[:8] + suffix for suffix in ('-a', '-b')]


def processes():
    lines = subprocess.check_output(['ps', '-axo', 'pid=,command='], text=True).splitlines()
    return [int(p[0]) for line in lines if len(p := line.strip().split(None, 1)) == 2 and p[1] == EXPECTED]


assert not processes(), 'Existing worktree server must be preserved; smoke requires a stopped instance'
OUTPUT.mkdir(parents=True, exist_ok=True)
evidence = {'instance': INFO, 'note_ids': IDS, 'samples': [], 'passed': False}
pid = None
pointer = front = None


def cli(action, note_id, *args):
    result = subprocess.run([CLI, action, '--id', note_id, *args], capture_output=True, text=True, timeout=25)
    assert result.returncode == 0, result.stdout + result.stderr
    state = json.loads(result.stdout)
    assert state['ok'] and state['instance'] == INFO['id'] and state['id'] == note_id, state
    return state


def wait_for(note_id, check):
    deadline = time.monotonic() + 6
    while True:
        state = cli('inspect', note_id)
        if check(state):
            return state
        assert time.monotonic() < deadline, state
        time.sleep(.15)


with tempfile.TemporaryDirectory(prefix='gibbon-updates-') as folder:
    helper = Path(folder) / 'native'
    source = helper.with_suffix('.swift')
    source.write_text(r'''
import AppKit
import CoreGraphics
import ApplicationServices
let a = CommandLine.arguments
if a[1] == "pointer" {
 let p = CGEvent(source:nil)!.location; print("\(p.x) \(p.y)")
} else if a[1] == "front" {
 print(NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0)
} else if a[1] == "activate" {
 NSRunningApplication(processIdentifier:Int32(a[2])!)?.activate(options:[])
} else if a[1] == "focus" {
 let app = AXUIElementCreateApplication(Int32(a[2])!)
 var value: CFTypeRef?
 AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value)
 for w in (value as? [AXUIElement] ?? []) {
  var title: CFTypeRef?
  AXUIElementCopyAttributeValue(w, kAXTitleAttribute as CFString, &title)
  if (title as? String) == a[3] {
   AXUIElementPerformAction(w, kAXRaiseAction as CFString)
   AXUIElementSetAttributeValue(w, kAXMainAttribute as CFString, kCFBooleanTrue)
   NSRunningApplication(processIdentifier:Int32(a[2])!)?.activate(options:[])
  }
 }
} else if a[1] == "move" || a[1] == "click" {
 let p = CGPoint(x:Double(a[2])!, y:Double(a[3])!)
 CGWarpMouseCursorPosition(p)
 CGEvent(mouseEventSource:nil, mouseType:.mouseMoved, mouseCursorPosition:p, mouseButton:.left)?.post(tap:.cghidEventTap)
 if a[1] == "click" {
  CGEvent(mouseEventSource:nil, mouseType:.leftMouseDown, mouseCursorPosition:p, mouseButton:.left)?.post(tap:.cghidEventTap)
  CGEvent(mouseEventSource:nil, mouseType:.leftMouseUp, mouseCursorPosition:p, mouseButton:.left)?.post(tap:.cghidEventTap)
 }
} else {
 let all = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as! [[String:Any]]
 let rows = all.filter { ($0[kCGWindowOwnerPID as String] as? NSNumber)?.intValue == Int(a[2])! }.map { w in
 ["title":w[kCGWindowName as String] ?? "", "id":w[kCGWindowNumber as String]!, "bounds":w[kCGWindowBounds as String]!] as [String:Any]
 }
 print(String(data:try! JSONSerialization.data(withJSONObject:rows), encoding:.utf8)!)
}
''')
    subprocess.run(['swiftc', str(source), '-o', str(helper)], check=True)

    def native(*args):
        if args[0] == 'focus':
            rows = json.loads(subprocess.check_output([str(helper), 'windows', str(args[1])], text=True))
            rect = next(w['bounds'] for w in rows if w['title'] == args[2])
            x, y = rect['X'] + rect['Width'] - 50, rect['Y'] + 15
            subprocess.run([str(helper), 'move', str(x), str(y)], check=True)
            time.sleep(.4)
            for _ in range(2):
                subprocess.run([str(helper), 'click', str(x), str(y)], check=True)
                time.sleep(.35)
            return ''
        return subprocess.check_output([str(helper), *map(str, args)], text=True).strip()

    def window(note_id):
        title = f"Agent Gibbon [{label}] — {note_id}"
        matches = [w for w in json.loads(native('windows', pid)) if w['title'] == title]
        assert len(matches) == 1, matches
        return matches[0]

    def sample(stage, note_id):
        state = cli('inspect', note_id)
        evidence['samples'].append({'stage': stage, 'state': state, 'windows': json.loads(native('windows', pid))})
        return state

    try:
        pointer = native('pointer').split()
        front = native('front')
        native('move', 5, 40)
        for note_id in IDS:
            state = cli('open', note_id, '--markdown', f'# {note_id}\n- [ ] A\n- [ ] B')
            assert note_id in state['note']['markdown']
        label = state['dev_label']
        matches = processes()
        assert len(matches) == 1, matches
        pid = matches[0]
        evidence['pid'] = pid
        files = subprocess.check_output(['/usr/sbin/lsof', '-a', '-p', str(pid), '-d', 'txt', '-Fn'], text=True).splitlines()
        assert 'n' + INFO['binary'] in files
        evidence['verified_executable'] = INFO['binary']
        time.sleep(1.5)
        for note_id in IDS:
            cli('update', note_id, '--markdown', f'# {note_id}\n- [x] A\n- [ ] B')
            wait_for(note_id, lambda s: s['highlight_revision'] is not None)
        state = sample('pending-diff', IDS[0])
        assert not state['feedback_surface'] or not state['feedback_surface']['visible']
        w = window(IDS[0]); r = w['bounds']
        native('move', r['X'] + r['Width']/2, r['Y'] + r['Height']/2)
        time.sleep(.5)
        assert cli('inspect', IDS[0])['highlight_revision'] is not None
        native('activate', pid)
        native('focus', pid, window(IDS[0])['title'])
        time.sleep(.5)
        focused_state = cli('inspect', IDS[0])
        assert focused_state['highlight_revision'] is not None, focused_state
        peer = window(IDS[1])['bounds']
        native('focus', pid, window(IDS[1])['title'])
        native('move', 5, 40)
        wait_for(IDS[0], lambda s: s['highlight_revision'] is None)
        assert cli('inspect', IDS[1])['highlight_revision'] is not None
        # Focus was intentionally moved to the peer to end the reading session.
        after = sample('focus-departure-acknowledged-peer-pending', IDS[0])
        for key in ('position', 'size', 'always_on_top', 'visible'):
            assert state[key] == after[key], key
        native('move', 5, 40)
        body = f'# {IDS[0]}\n- [x] A\n- [x] B'
        state = cli('update', IDS[0], '--markdown', body, '--change-mode', 'replace', '--highlight', 'newly-completed')
        assert state['note']['completion_html'] and state['highlight_revision'] is None
        assert not state['feedback_active']
        native('move', r['X'] + r['Width']/2, r['Y'] + r['Height']/2)
        time.sleep(.5)
        state = sample('quiet-completion-survives-hover', IDS[0])
        assert state['note']['completion_html']
        assert not state['feedback_surface'] or not state['feedback_surface']['visible']
        subprocess.run(['screencapture', '-x', '-l', str(w['id']), str(OUTPUT / 'completion.png')], check=True)
        # A real focus change while the pointer is outside verifies the focus path.
        native('activate', pid)
        native('focus', pid, window(IDS[0])['title'])
        time.sleep(.2)
        native('move', 5, 40)
        wait_for(IDS[0], lambda s: s['focused'])
        cli('update', IDS[0], '--markdown', body + '\nfocused update')
        time.sleep(.6)
        focused_state = cli('inspect', IDS[0])
        assert focused_state['highlight_revision'] is not None, focused_state
        peer = window(IDS[1])['bounds']
        native('focus', pid, window(IDS[1])['title'])
        native('move', 5, 40)
        wait_for(IDS[0], lambda s: s['highlight_revision'] is None and s['note']['completion_html'] is None)
        sample('focus-departure-clears-diff-and-completion', IDS[0])
        cli('close', IDS[1])
        cli('update', IDS[1], '--markdown', 'hidden changed')
        hidden = sample('hidden-update-preserved', IDS[1])
        assert not hidden['visible'] and hidden['highlight_revision'] is not None
        evidence['passed'] = True
        print(f'PASS: rollover retained, focus departure clears, focused update retained, quiet completion, peer/hidden/geometry preservation; PID={pid}')
    finally:
        matches = processes()
        if pid is None and len(matches) == 1:
            pid = matches[0]
        if pointer:
            native('move', *pointer)
        if front:
            native('activate', front)
        if pid in processes():
            os.kill(pid, signal.SIGTERM)
            for _ in range(40):
                if pid not in processes():
                    break
                time.sleep(.1)
        evidence['test_server_stopped'] = not processes()
        (OUTPUT / 'evidence.json').write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + '\n')
