#!/usr/bin/env python3
"""macOS rollover regression in a stopped, isolated development instance.

Requires this worktree's debug build. Uses only its development CLI, seeds a
temporary 35% setting, restores that setting and pointer, and stops only the
exact server started here. Existing instances and their notes are untouched.
"""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import uuid

assert sys.platform == 'darwin', 'macOS native check only'
ROOT = Path(__file__).resolve().parents[1]
INFO = json.loads(subprocess.check_output(['node', str(ROOT / 'scripts/dev.mjs'), 'info'], text=True))
BIN = str(ROOT / 'scripts/agent-gibbon-dev')
EXPECTED = INFO['binary'] + ' --serve'
OUTPUT = ROOT / 'docs/verification/hover-opacity'
SETTING = Path.home() / 'Library/Application Support/org.egoing.floatnote/appearance' / INFO['id'] / 'opacity.json'
PREFIX = 'hover-opacity-' + uuid.uuid4().hex[:8]
IDS = [PREFIX + '-a', PREFIX + '-b']


def processes():
    lines = subprocess.check_output(['ps', '-axo', 'pid=,command='], text=True).splitlines()
    return [int(parts[0]) for line in lines if len(parts := line.strip().split(None, 1)) == 2 and parts[1] == EXPECTED]


assert Path(INFO['binary']).is_file(), 'Run npm run dev:build first'
assert not processes(), 'Stop this worktree instance before running; existing apps are never stopped'
OUTPUT.mkdir(parents=True, exist_ok=True)
saved = SETTING.read_bytes() if SETTING.exists() else None
evidence = {'instance': INFO, 'note_ids': IDS, 'samples': [], 'passed': False}
pid = None
pointer = None
created = []


def cli(action, note_id, *args):
    result = subprocess.run([BIN, action, '--id', note_id, *args], text=True, capture_output=True, timeout=25)
    assert result.returncode == 0, result.stdout + result.stderr
    data = json.loads(result.stdout)
    assert data['ok'] and data['instance'] == INFO['id'] and data['id'] == note_id, data
    return data


with tempfile.TemporaryDirectory(prefix='gibbon-opacity-hover-') as folder:
    helper = Path(folder) / 'native'
    source = helper.with_suffix('.swift')
    source.write_text(r'''
import AppKit
import CoreGraphics
import Foundation
let a = CommandLine.arguments
if a[1] == "pointer" {
 let p = CGEvent(source:nil)!.location
 print("\(p.x) \(p.y)")
} else if a[1] == "front" {
 print(NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0)
} else if a[1] == "move" {
 let p = CGPoint(x:Double(a[2])!, y:Double(a[3])!)
 CGWarpMouseCursorPosition(p)
 CGEvent(mouseEventSource:nil, mouseType:.mouseMoved, mouseCursorPosition:p, mouseButton:.left)?.post(tap:.cghidEventTap)
} else {
 let all = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as! [[String:Any]]
 let filtered = all.filter { ($0[kCGWindowOwnerPID as String] as? NSNumber)?.intValue == Int(a[2])! }.map { w in
  ["title":w[kCGWindowName as String] ?? "", "alpha":w[kCGWindowAlpha as String]!, "id":w[kCGWindowNumber as String]!, "bounds":w[kCGWindowBounds as String]!] as [String:Any]
 }
 print(String(data:try! JSONSerialization.data(withJSONObject:filtered), encoding:.utf8)!)
}
''')
    subprocess.run(['swiftc', str(source), '-o', str(helper)], check=True)
    ax_helper = Path(folder) / 'window-ax'
    subprocess.run(['swiftc', str(ROOT / 'scripts/design-system-ax.swift'), '-o', str(ax_helper)], check=True)

    def native(*args):
        return subprocess.check_output([str(helper), *map(str, args)], text=True).strip()

    def move(x, y):
        native('move', x, y)
        actual = list(map(float, native('pointer').split()))
        assert abs(actual[0] - x) < 2 and abs(actual[1] - y) < 2, 'Pointer movement unavailable'

    def windows():
        return json.loads(native('windows', pid))

    def window(note_id, feedback=False):
        title = f"Agent Gibbon [{label}] — {note_id}" + (' · Feedback' if feedback else '')
        matches = [w for w in windows() if w['title'] == title]
        assert len(matches) == 1, (title, windows())
        return matches[0]

    def alpha(note_id, value, feedback=False):
        actual = window(note_id, feedback)
        assert abs(actual['alpha'] - value) < .015, (note_id, value, actual)
        return actual

    def snapshot(stage):
        # Native sampling precedes inspect, which can itself synchronize opacity.
        sample = {'stage': stage, 'windows': windows(), 'front_pid': native('front')}
        sample['notes'] = [{k: s[k] for k in ('id', 'instance', 'opacity', 'global_opacity', 'hovered', 'visible', 'focused', 'position', 'size', 'always_on_top', 'highlight_revision')}
                           for s in [cli('inspect', note_id) for note_id in IDS]]
        evidence['samples'].append(sample)
        assert sample['front_pid'] == front, 'Rollover changed foreground app'
        assert SETTING.read_bytes() == seeded, 'Pointer contact changed stored opacity'
        return sample

    def stable(before, after):
        for before_note, after_note in zip(before['notes'], after['notes']):
            for key in ('position', 'size', 'visible', 'focused', 'always_on_top'):
                assert before_note[key] == after_note[key], (key, before_note, after_note)

    try:
        pointer = native('pointer').split()
        front = native('front')
        SETTING.parent.mkdir(parents=True, exist_ok=True)
        seeded = b'{"percent":35}'
        SETTING.write_bytes(seeded)
        for note_id in IDS:
            state = cli('open', note_id, '--title', 'Rollover native verification', '--markdown', f'# {note_id}\n\nHover opacity verification 20261106')
            assert note_id in state['note']['markdown']
            created.append(note_id)
        label = state['dev_label']
        matches = processes()
        assert len(matches) == 1, matches
        pid = matches[0]
        evidence['pid'] = pid
        evidence['binary_command'] = EXPECTED
        executable_files = subprocess.check_output(['/usr/sbin/lsof', '-a', '-p', str(pid), '-d', 'txt', '-Fn'], text=True).splitlines()
        assert 'n' + INFO['binary'] in executable_files, 'Unexpected server executable'
        evidence['verified_executable'] = INFO['binary']
        move(5, 40)
        time.sleep(3.2)
        # Automatic placement is adjacent: the first note's Feedback can overlap
        # the second memo. Put this test-only peer elsewhere so pointer contact
        # has one unambiguous owner when checking isolation.
        peer_title = f"Agent Gibbon [{label}] — {IDS[1]}"
        subprocess.run([str(ax_helper), str(pid), peer_title, 'position', '60', '160'], check=True)
        time.sleep(.3)
        for note_id in IDS:
            alpha(note_id, .35)
        baseline = snapshot('outside-35')
        rect = window(IDS[0])['bounds']
        inside = (rect['X'] + rect['Width'] / 2, rect['Y'] + rect['Height'] / 2)
        move(*inside)
        time.sleep(.35)
        hovered_window = alpha(IDS[0], 1)
        alpha(IDS[1], .35)
        snapshot('hover-one-peer-unchanged')
        subprocess.run(['screencapture', '-x', '-l', str(hovered_window['id']), str(OUTPUT / 'hovered.png')], check=True)
        move(5, 40)
        time.sleep(.6)
        alpha(IDS[0], 1)
        snapshot('leave-delay')
        time.sleep(2.2)
        alpha(IDS[0], .35)
        stable(baseline, snapshot('leave-restored-35'))
        cli('update', IDS[0], '--markdown', f'# {IDS[0]}\n\nChanged verification body')
        time.sleep(.7)
        surface = cli('inspect', IDS[0])['feedback_surface']
        assert surface and surface['visible'], surface
        alpha(IDS[0], .35)
        alpha(IDS[0], .35, True)
        feedback_baseline = snapshot('feedback-outside-35')
        rect = window(IDS[0], True)['bounds']
        move(rect['X'] + rect['Width'] / 2, rect['Y'] + rect['Height'] / 2)
        time.sleep(.35)
        alpha(IDS[0], 1)
        alpha(IDS[0], 1, True)
        alpha(IDS[1], .35)
        assert not cli('inspect', IDS[0])['hovered'], 'Expected contact only with Feedback'
        snapshot('feedback-only-hover-both-100')
        move(5, 40)
        time.sleep(2.25)
        move(*inside)
        time.sleep(.3)
        alpha(IDS[0], 1)
        alpha(IDS[0], 1, True)
        snapshot('reenter-interrupts-fade')
        move(5, 40)
        time.sleep(2.9)
        alpha(IDS[0], .35)
        alpha(IDS[0], .35, True)
        after = snapshot('both-restored-35')
        stable(feedback_baseline, after)
        cli('close', IDS[1])
        cli('update', IDS[1], '--markdown', 'Hidden peer update')
        assert not cli('inspect', IDS[1])['visible']
        evidence['passed'] = True
        print(f"PASS: native 35→100→35, peer isolation, Feedback hover, fade interruption, saved value/focus/geometry/hidden preservation; PID={pid}", flush=True)
    finally:
        # Even a failed first open can have started a server; it is ours because
        # no matching process existed before this fixture was created.
        if pid is None:
            matches = processes()
            if len(matches) == 1:
                pid = matches[0]
        for note_id in created:
            cli('close', note_id)
        if pointer:
            native('move', *pointer)
        if pid in processes():
            os.kill(pid, signal.SIGTERM)
            for _ in range(50):
                if pid not in processes():
                    break
                time.sleep(.1)
            assert pid not in processes(), 'Test server did not stop'
        if saved is None:
            SETTING.unlink(missing_ok=True)
        else:
            SETTING.write_bytes(saved)
        evidence['test_server_stopped'] = not processes()
        evidence['setting_restored'] = (SETTING.read_bytes() if SETTING.exists() else None) == saved
        (OUTPUT / 'evidence.json').write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + '\n')
