#!/usr/bin/env python3
"""macOS native-alpha regression; requires a built, stopped dev checkout.

Uses only the selected checkout's development CLI. Restores its opacity setting
and the pointer, and stops only the exact process started by this test.
"""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import uuid


def run(*args):
    return subprocess.check_output(list(map(str, args)), text=True).strip()


def main():
    assert sys.platform == 'darwin', 'macOS native check only'
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--report-dir', type=Path, required=True)
    parser.add_argument('--diagnostic-continue', action='store_true',
                        help='record alpha mismatches and continue; still exits nonzero')
    args = parser.parse_args()
    root, output = args.root.resolve(), args.report_dir.resolve()
    info = json.loads(run('node', root / 'scripts/dev.mjs', 'info'))
    cli_path = root / 'scripts/agent-gibbon-dev'
    expected_process = info['binary'] + ' --serve'

    def processes():
        return [int(p[0]) for line in run('ps', '-axo', 'pid=,command=').splitlines()
                if len(p := line.strip().split(None, 1)) == 2 and p[1] == expected_process]

    assert not processes(), 'Existing dev apps are never stopped; use a separate source snapshot'
    output.mkdir(parents=True, exist_ok=True)
    setting = Path.home() / 'Library/Application Support/org.egoing.floatnote/appearance' / info['id'] / 'opacity.json'
    original_setting = setting.read_bytes() if setting.exists() else None
    evidence = {'origin': info, 'cases': [], 'passed': False, 'alpha_mismatches': []}
    pid = None
    original_pointer = None
    with tempfile.TemporaryDirectory(prefix='gibbon-spotlight-native-') as folder:
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

        def native(*args):
            return run(helper, *args)

        def move(x, y):
            native('move', x, y)
            actual = list(map(float, native('pointer').split()))
            assert abs(actual[0] - x) < 2 and abs(actual[1] - y) < 2

        original_pointer = list(map(float, native('pointer').split()))
        original_front = native('front')
        try:
            for manual in [None, 35]:
                case = {'manual_percent': manual, 'samples': []}
                evidence['cases'].append(case)
                if manual is None:
                    setting.unlink(missing_ok=True)
                else:
                    setting.parent.mkdir(parents=True, exist_ok=True)
                    setting.write_text(json.dumps({'percent': manual}))
                seeded = setting.read_bytes() if setting.exists() else None
                prefix = 'spotlight-' + uuid.uuid4().hex[:8]
                ids = []

                def cli(action, note, *extra):
                    state = json.loads(run(cli_path, action, '--id', note, *extra))
                    assert state['ok'] and state['instance'] == info['id'] and state['id'] == note, state
                    assert prefix in state['note']['markdown'], state
                    return state

                def opened(suffix):
                    nonlocal pid
                    note = prefix + '-' + suffix
                    state = cli('open', note, '--markdown', f'# {prefix} {suffix}\n' + 'Spotlight verification\n\n' * 45)
                    ids.append(note)
                    found = processes()
                    assert len(found) == 1, found
                    if pid is None:
                        pid = found[0]
                        case['pid'] = pid
                        case['executable'] = run('ps', '-p', pid, '-o', 'comm=')
                        assert case['executable'] == info['binary']
                        case['lsof'] = run('lsof', '-a', '-p', pid, '-d', 'txt', '-Fn')
                        assert 'n' + info['binary'] in case['lsof'].splitlines()
                    assert found == [pid]
                    return state

                def snapshot(stage, expected, holding=None):
                    # Read actual alpha BEFORE inspect can synchronize the window.
                    windows = json.loads(native('windows', pid))
                    sample = {'stage': stage, 'windows': windows, 'notes': [],
                              'pointer': native('pointer'), 'front_pid': native('front')}
                    case['samples'].append(sample)
                    for note, alpha in expected.items():
                        state = cli('inspect', note)
                        title = f"Agent Gibbon [{state['dev_label']}] — {note}"
                        matches = [w for w in windows if w['title'] == title]
                        assert len(matches) == 1, (title, windows)
                        sample['notes'].append({k: state[k] for k in (
                            'id', 'instance', 'opacity', 'hovered', 'visible', 'focused',
                            'always_on_top', 'position', 'size', 'animating', 'layout_pending')})
                        if abs(matches[0]['alpha'] - alpha) >= .015:
                            mismatch = {'stage': stage, 'note': note, 'expected': alpha,
                                        'actual': matches[0], 'manual_percent': manual}
                            evidence['alpha_mismatches'].append(mismatch)
                            assert args.diagnostic_continue, mismatch
                        assert state['visible'] and not state['focused'] and not state['hovered'], sample
                        if note == holding:
                            assert state['animating'], 'Pointer moved and ended the stationary entrance hold'
                    assert sample['front_pid'] == case['front_pid'], 'Show changed foreground app'
                    assert (setting.read_bytes() if setting.exists() else None) == seeded
                    print('OBSERVED:' if evidence['alpha_mismatches'] else 'PASS:', manual, stage, flush=True)
                    return sample

                # Complete startup/restoration of saved hidden WebViews before
                # anchoring the stationary-pointer measurements.
                opened('seed')
                cli('close', ids.pop())
                case['front_before_startup'] = original_front
                case['front_pid'] = native('front')
                move(650, 500)
                opened('a')
                a = ids[0]
                snapshot('new-at-cursor', {a: 1.0})
                time.sleep(3.0)
                snapshot('stationary-hold-over-two-seconds', {a: 1.0}, holding=a)
                move(620, 500)
                # No inspect, pointer events or CLI commands during landing/fade.
                time.sleep(3.5)
                idle = .5 if manual is None else manual / 100
                before = snapshot('arrived-fades-without-more-input', {a: idle})
                opened('b')
                b = ids[-1]
                shown = snapshot('new-opaque-existing-reflow-dim', {a: idle, b: 1.0})
                title = f"Agent Gibbon [{cli('inspect', b)['dev_label']}] — {b}"
                window = next(w for w in shown['windows'] if w['title'] == title)
                capture = output / ('auto-new.png' if manual is None else 'manual-new.png')
                subprocess.run(['screencapture', '-x', '-l', str(window['id']), str(capture)], check=True)
                case['capture'] = {'path': str(capture), 'pid': pid, 'window': window, 'note': b}
                time.sleep(3.0)
                snapshot('only-new-memo-holds-attention', {a: idle, b: 1.0}, holding=b)
                move(590, 500)
                time.sleep(3.5)
                snapshot('both-return-to-idle', {a: idle, b: idle})
                cli('close', b)
                hidden = cli('update', b, '--markdown', f'# {prefix} hidden update')
                assert not hidden['visible']
                cli('open', b)
                snapshot('hidden-restore-starts-opaque', {a: idle, b: 1.0})
                opened('c')
                opened('d')
                c, d = ids[-2:]
                time.sleep(3.2)
                snapshot('rapid-open-latest-spotlight', {a: idle, b: idle, c: idle, d: 1.0}, holding=d)
                assert before['notes'][0]['always_on_top'] == shown['notes'][0]['always_on_top']
                for note in ids:
                    cli('close', note)
                assert run('ps', '-p', pid, '-o', 'comm=') == info['binary']
                os.kill(pid, signal.SIGTERM)
                for _ in range(100):
                    if not processes():
                        break
                    time.sleep(.05)
                assert not processes()
                pid = None
            assert not evidence['alpha_mismatches'], evidence['alpha_mismatches']
            evidence['passed'] = True
        except BaseException as error:
            evidence['error'] = repr(error)
            if isinstance(error, subprocess.CalledProcessError):
                evidence['command_output'] = error.output
                print(error.output, file=sys.stderr)
            raise
        finally:
            if pid is not None and processes() == [pid]:
                os.kill(pid, signal.SIGTERM)
                for _ in range(100):
                    if not processes():
                        break
                    time.sleep(.05)
            if original_pointer:
                move(*original_pointer)
            if original_setting is None:
                setting.unlink(missing_ok=True)
            else:
                setting.parent.mkdir(parents=True, exist_ok=True)
                setting.write_bytes(original_setting)
            evidence['server_stopped'] = not processes()
            evidence['setting_restored'] = (setting.read_bytes() if setting.exists() else None) == original_setting
            (output / 'evidence.json').write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + '\n')


if __name__ == '__main__':
    main()
