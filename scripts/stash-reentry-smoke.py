#!/usr/bin/env python3
"""macOS #54: PID-bound native menu, real pointer, repeated immediate reentry.

Run with before/after on a built, otherwise idle worktree dev instance. Never
uses the global modifier chord: another instance may own it. Leaves test notes
hidden and restores the pointer. Reports include failed runs and native origin.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
import time
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('phase', choices=['before', 'after'])
args = parser.parse_args()
OUT = ROOT / 'docs/verification' / ('issue54-' + args.phase)
OUT.mkdir(parents=True, exist_ok=True)


def run(*values):
    return subprocess.check_output(list(map(str, values)), text=True, timeout=30).strip()


info = json.loads(run('node', ROOT / 'scripts/dev.mjs', 'info'))
assert info['root'] == str(ROOT)
id = 'issue54-' + uuid.uuid4().hex[:8]
events = []
report = {'origin': info, 'id': id, 'phase': args.phase, 'events': events, 'result': 'failed',
          'head': run('git', '-C', ROOT, 'rev-parse', 'HEAD'),
          'binary_sha256': hashlib.sha256(Path(info['binary']).read_bytes()).hexdigest(),
          'source_sha256': hashlib.sha256((ROOT / 'src-tauri/src/stash.rs').read_bytes()).hexdigest()}
began = time.monotonic()


def cli(action, *values):
    result = json.loads(run(ROOT / 'scripts/agent-gibbon-dev', action, '--id', id, *values))
    assert result['ok'] and result['instance'] == info['id'], result
    return result


def state(label):
    result = cli('inspect')
    assert result['id'] == id and result['note']['markdown'] == id
    events.append({'at': time.monotonic() - began, 'label': label, 'stash': result['stash']})
    return result['stash']


def wait(predicate, label, timeout=5):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        current = state(label)
        if predicate(current):
            return current
        time.sleep(.025)
    raise AssertionError((label, current))


with tempfile.TemporaryDirectory(prefix='gibbon-issue54-') as directory:
    temp = Path(directory)
    for source, name in [('entrance-pointer.swift', 'pointer'), ('auto-layout-native.swift', 'native'),
                         ('macos-input.swift', 'mouse'),
                         ('frontmost.swift', 'front')]:
        subprocess.run(['swiftc', str(ROOT / 'scripts' / source), '-o', str(temp / name)], check=True)

    def helper(name, *values):
        return run(temp / name, *values)

    original = json.loads(helper('pointer'))

    def move(x, y, label):
        events.append({'at': time.monotonic() - began, 'label': label, 'pointer': [x, y]})
        helper('pointer', 'move', x, y)

    def center(rect, label):
        move(rect['x'] + rect['width'] - 24, rect['y'] + 24, label)

    # Only this worktree may be started. Refuse to disturb its existing stash.
    try:
        cli('open', '--markdown', id)
        proof = cli('inspect')
        assert not proof['stash']['active'], 'existing stash must be restored by its owner first'
        processes = [line.strip().split(None, 1) for line in run('ps', '-axo', 'pid=,command=').splitlines()]
        matches = [int(row[0]) for row in processes if len(row) == 2 and row[1] == info['binary'] + ' --serve']
        assert len(matches) == 1, matches
        pid = matches[0]
        title = 'Agent Gibbon [' + proof['dev_label'] + '] — '
        report.update(pid=pid, executable=info['binary'], proof=proof)

        def native():
            return json.loads(helper('native', pid))

        time.sleep(1.1)
        move(original['x'] + 16, original['y'], 'finish memo entrance')
        wait(lambda s: not cli('inspect')['animating'], 'memo entrance complete')
        # Some macOS callers cannot read CG window names or AXWindows (#53).
        # Match native PID/bounds, then require Orca's independent exact title.
        def surface(rect, expected_title, screenshot=False):
            rows = native()['windows']
            candidates = [w for w in rows if w.get('kCGWindowLayer') == 5 and all(
                abs(w['kCGWindowBounds'][key] - rect[field]) < 2
                for key, field in [('X', 'x'), ('Y', 'y'), ('Width', 'width'), ('Height', 'height')])]
            assert len(candidates) == 1, (rect, rows)
            window = candidates[0]
            command = ['orca', 'computer', 'get-app-state', '--app', f'pid:{pid}',
                       '--window-id', str(window['kCGWindowNumber']), '--json']
            if not screenshot:
                command.append('--no-screenshot')
            result = json.loads(run(*command))
            assert result['ok'], result
            assert result['result']['snapshot']['window']['title'] == expected_title, result
            return result

        settled = cli('inspect')
        origin = native()
        # This probe compares physical CLI geometry to logical CG bounds. Refuse
        # mixed-scale ambiguity rather than identifying a window on the wrong screen.
        scales = {screen['scale'] for screen in origin['screens']}
        assert len(scales) == 1, 'mixed-DPI origin matching requires a separate native probe'
        scale = scales.pop()
        rect = {key: settled['position'][key] / scale for key in ['x', 'y']}
        rect.update({key: settled['size'][key] / scale for key in ['width', 'height']})
        report['memo_surface'] = surface(rect, title + id)
        report['native_origin'] = origin
        report['before_stash'] = settled
    except BaseException as error:
        report['error'] = str(error)
        try:
            cli('close')
        finally:
            move(original['x'], original['y'], 'restore pointer after setup failure')
            (OUT / 'report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2))
        raise

    def toggle():
        prefix = f'tell application "System Events" to tell (first application process whose unix id is {pid}) to '
        position = run('osascript', '-e', prefix + 'get {position, size} of menu bar item 1 of menu bar 2')
        x, y, width, height = map(float, position.split(','))
        move(x - 100, y + 100, 'menu approach')
        move(x + width / 2, y + height / 2, 'PID-owned tray')
        time.sleep(.15)
        helper('mouse', 'click', x + width / 2, y + height / 2)
        menu = prefix + 'tell menu 1 of menu bar item 1 of menu bar 2 to '
        assert run('osascript', '-e', menu + 'get name of menu item 1') == info['name']
        run('osascript', '-e', menu + 'click (first menu item whose name starts with "숨김")')

    def shot(name):
        result = surface(state('capture origin')['indicator'], title + 'stash-indicator', True)
        (OUT / (name + '-source.json')).write_text(json.dumps(result, ensure_ascii=False, indent=2))
        shutil.copyfile(result['result']['screenshot']['path'], OUT / (name + '.png'))

    try:
        toggle()
        initial = wait(lambda s: s['active'] and not s['animating'] and s['tip_visible'], 'initial hint')
        bottom = initial['indicator']
        # Stay on the same display, well outside both mascot locations.
        away = (bottom['x'] + 40, bottom['y'] + 24)
        move(*away, 'outside 96px')
        time.sleep(.12)
        focus = helper('front')
        report['focus_before'] = focus
        center(bottom, 'first approach during initial hint')
        first_started = time.monotonic()
        wait(lambda s: s['indicator_moving'] and not s['tip_visible'], 'first flight')
        first = wait(lambda s: not s['indicator_moving'] and s['tip_visible'], 'first arrival')
        top = first['indicator']
        assert top['y'] != bottom['y']
        if args.phase == 'before':
            shot('first-arrival')
        move(*away, 'exit during arrival hint')
        time.sleep(.12)
        center(top, 'second approach without 2s or 6s wait')
        if args.phase == 'before':
            time.sleep(.4)
            blocked = state('blocked second approach')
            assert not blocked['indicator_moving'] and blocked['indicator'] == top
            result = 'reproduced: valid reentry blocked during arrival hint'
        else:
            wait(lambda s: s['indicator_moving'] and not s['tip_visible'], 'second flight', 1)
            assert time.monotonic() - first_started < 2, 'reentry must precede old flight-start cooldown'
            second_started = time.monotonic()
            # Follow during flight: arriving under a stationary pointer must not
            # arm or queue an escape. A fresh post-arrival exit is required.
            center(bottom, 'follow moving icon to its destination')
            second = wait(lambda s: not s['indicator_moving'] and s['tip_visible'], 'second arrival')
            assert second['indicator'] == bottom
            time.sleep(.5)
            assert state('held pointer does not chase')['indicator'] == bottom
            move(*away, 'fresh post-arrival exit')
            time.sleep(.12)
            center(bottom, 'third approach during hint')
            wait(lambda s: s['indicator_moving'] and not s['tip_visible'], 'third flight', 1)
            assert time.monotonic() - second_started < 2, 'third flight must precede old cooldown'
            move(*away, 'leave moving icon')
            third = wait(lambda s: not s['indicator_moving'] and s['tip_visible'], 'third arrival')
            assert third['indicator'] == top
            shot('third-arrival')
            # The six-second hint is still displayed when no new approach occurs.
            wait(lambda s: not s['tip_visible'], 'hint expires independently', 8)
            report['focus_after'] = helper('front')
            assert report['focus_after'] == focus, 'frontmost PID changed during escape observation'
            result = 'passed'
        report['focus_before'] = focus
        report['focus_after'] = helper('front')
        assert report['focus_after'] == focus
        toggle()
        wait(lambda s: not s['active'], 'restored')
        restored = cli('inspect')
        assert restored['visible'] and restored['always_on_top'] == proof['always_on_top']
        for field in ['position', 'size', 'folded']:
            assert restored[field] == settled[field], (field, settled[field], restored[field])
        report['restored'] = restored
        report['result'] = result
    except BaseException as error:
        report['error'] = str(error)
        raise
    finally:
        try:
            if cli('inspect')['stash']['active']:
                toggle()
                wait(lambda s: not s['active'], 'cleanup restore')
            cli('close')
        finally:
            move(original['x'], original['y'], 'restore pointer')
            (OUT / 'report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2))
            print(OUT)
