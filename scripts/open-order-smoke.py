#!/usr/bin/env python3
"""macOS live regression for #34; run against a worker-owned dev PID only.

Creates unique test notes, uses this checkout's dev CLI, and records PID/title/
body-bound Orca computer-use captures. Never stops an app or changes other notes.
Requires Swift, Orca, Accessibility and event-posting/screenshot permissions.
"""
import argparse
import json
from pathlib import Path
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / 'scripts/agent-gibbon-dev'


def run(*args):
    return subprocess.check_output(list(map(str, args)), text=True)


def front():
    return run('osascript', '-e', 'tell application "System Events" to get unix id of first application process whose frontmost is true').strip()


def frame(state):
    return tuple(state['position'][k] for k in ('x', 'y')) + tuple(state['size'][k] for k in ('width', 'height'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pid', type=int, required=True, help='dev PID started by this worker')
    parser.add_argument('--report-dir', type=Path, required=True)
    args = parser.parse_args()
    folder = args.report_dir.resolve()
    folder.mkdir(parents=True, exist_ok=True)
    info = json.loads(run('node', ROOT / 'scripts/dev.mjs', 'info'))
    assert run('ps', '-p', args.pid, '-o', 'comm=').strip() == info['binary']
    log = {'origin': info, 'pid': args.pid, 'checks': [], 'states': {}, 'captures': {}, 'opens': []}
    prefix = 'open34-' + uuid.uuid4().hex[:6]
    ids, windows = [], {}
    original_front = front()
    pointer, native = folder / 'pointer', folder / 'native'
    for source, target in [('entrance-pointer.swift', pointer), ('auto-layout-native.swift', native)]:
        subprocess.run(['swiftc', str(ROOT / 'scripts' / source), '-o', str(target)], check=True)
    original_pointer = json.loads(run(pointer))

    def move(x, y):
        run(pointer, 'move', x, y)

    def cli(action, note, *extra):
        state = json.loads(run(CLI, action, '--id', note, *extra))
        assert state['ok'], state
        if action == 'inspect':
            assert state['instance'] == info['id'] and state['id'] == note, state
            assert prefix in state['note']['markdown'], state
        return state

    def inspect(note):
        return cli('inspect', note)

    def settled():
        for _ in range(100):
            states = [inspect(note) for note in ids]
            if all(not s['animating'] and not s['layout_pending'] for s in states):
                return states
            time.sleep(.1)
        raise AssertionError('layout did not settle')

    def check(name):
        log['checks'].append(name)
        print('PASS:', name, flush=True)

    def capture(note, name):
        state = inspect(note)
        # Orca list-windows can omit borderless Tauri windows. Resolve their
        # CGWindow ID by exact PID/title, then target that ID through Orca.
        listing = json.loads(run(native, args.pid))
        title = f"Agent Gibbon [{state['dev_label']}] — {note}"
        matches = [w for w in listing['windows'] if w.get('kCGWindowName') == title]
        assert len(matches) == 1, (title, listing)
        windows[note] = matches[0]['kCGWindowNumber']
        selector = ['--window-id', str(windows[note])]
        result = json.loads(run('orca', 'computer', 'get-app-state', '--app', f'pid:{args.pid}', *selector, '--max-elements', '45', '--json'))
        assert result['ok'], result
        snapshot = result['result']['snapshot']
        assert snapshot['app']['pid'] == args.pid
        assert snapshot['window']['title'].endswith(' — ' + note), snapshot
        assert prefix in snapshot['treeText'], snapshot['treeText']
        windows[note] = snapshot['window']['id']
        image = result['result']['screenshot']
        (folder / (name + '.png')).write_bytes(Path(image['path']).read_bytes())
        (folder / (name + '-gui.json')).write_text(json.dumps(result, ensure_ascii=False, indent=2))
        log['captures'][name] = {'note': note, 'window': snapshot['window'], 'inspect': state}

    def open_note(suffix, rapid=False):
        note = prefix + '-' + suffix
        ids.append(note)
        move(650, 650)
        before_front = front()
        before_focus = {n: inspect(n)['focused'] for n in ids[:-1]}
        response = cli('open', note, '--markdown', f'# {prefix} {suffix}\n' + '본문 순서 검증\n\n' * 45)
        log['opens'].append({'id': note, 'rapid': rapid, 'animating': response['animating'], 'pending': response['layout_pending']})
        move(700, 650)
        assert front() == before_front
        assert before_focus == {n: inspect(n)['focused'] for n in ids[:-1]}
        assert not inspect(note)['focused']
        if not rapid:
            settled()
            capture(note, 'opened-' + suffix)
        return note

    def assert_order(expected, name):
        states = {n: inspect(n) for n in expected}
        log['states'][name] = states
        assert all(s['visible'] and not s['auto_layout_excluded'] for s in states.values())
        actual = sorted(expected, key=lambda n: (-states[n]['position']['x'], states[n]['position']['y']))
        assert actual == expected, (actual, expected)
        for i, note in enumerate(expected):
            x, y, w, h = frame(states[note])
            for other in expected[:i]:
                X, Y, W, H = frame(states[other])
                assert not (x < X + W and X < x + w and y < Y + H and Y < y + h)

    try:
        expected = [open_note(suffix) for suffix in ('z', 'y', 'x', 'w', 'v', 'u')]
        assert_order(expected, 'sequential')
        assert len({inspect(n)['position']['x'] for n in expected}) >= 2
        assert front() == original_front
        check('reverse IDs: sequential open order, left-column wrap, no overlap, focus preserved')
        first = expected[0]
        before = {n: frame(inspect(n)) for n in expected}
        cli('open', first)
        cli('update', first, '--markdown', f'# {prefix} visible update\n' + '본문\n\n' * 45)
        settled()
        assert before == {n: frame(inspect(n)) for n in expected}
        check('visible reopen/content update preserve order and geometry')

        hidden = expected.pop(1)
        cli('close', hidden)
        saved = frame(inspect(hidden))
        cli('update', hidden, '--markdown', f'# {prefix} hidden update\n' + '숨김 본문\n\n' * 45)
        assert not inspect(hidden)['visible'] and frame(inspect(hidden)) == saved
        appended = open_note('t')
        expected.append(appended)
        assert not inspect(hidden)['visible']
        cli('open', hidden)
        move(740, 650)
        settled()
        expected.append(hidden)
        assert_order(expected, 'restored-tail')
        capture(hidden, 'restored-tail')
        check('hidden update stays hidden; restore joins tail and preserves other relative order')

        for suffix in ('s', 'r', 'q'):
            expected.append(open_note(suffix, rapid=True))
        assert any(s['animating'] for s in log['opens'] if s['rapid'])
        settled()
        assert_order(expected, 'rapid')
        capture(first, 'after-first')
        capture(expected[-1], 'after-last')
        assert front() == original_front
        check('rapid opens retarget transitions without reordering; focus preserved')

        # Real native drag: Tauri start_dragging reads the hardware pointer.
        # Orca supplies the PID/window/body-bound before/after GUI evidence.
        mouse = folder / 'mouse'
        subprocess.run(['swiftc', str(ROOT / 'scripts/macos-input.swift'), '-o', str(mouse)], check=True)
        state = inspect(first)
        scale = original_pointer['scale']
        x, y, w, h = [v / scale for v in frame(state)]
        move(x + 40, y + 70)
        run(mouse, 'drag', x + 40, y + 70, 1200, 500)
        time.sleep(.3)
        obstacle = inspect(first)
        assert obstacle['auto_layout_excluded'] and frame(obstacle) != frame(state), obstacle
        capture(first, 'manual-obstacle')
        expected.remove(first)
        focus_before = front()
        expected.append(open_note('p'))
        settled()
        assert_order(expected, 'manual-excluded')
        assert frame(inspect(first)) == frame(obstacle)
        X, Y, W, H = frame(obstacle)
        for note in expected:
            x, y, w, h = frame(inspect(note))
            assert not (x < X + W and X < x + w and y < Y + H and Y < y + h)
        assert front() == focus_before
        capture(first, 'manual-preserved')
        check('native manual drag excluded; new open preserves obstacle geometry and current user focus')
        cli('update', first, '--markdown', f'# {prefix} re-included\n' + '본문\n\n' * 45)
        assert not inspect(first)['auto_layout_excluded']
        assert frame(inspect(first)) == frame(obstacle)
        expected.insert(0, first)
        expected.append(open_note('o'))
        assert_order(expected, 're-included')
        check('content action re-includes at original relative order without immediate geometry change')
        cli('close', first)
        expected.remove(first)
        remaining = expected.pop(0)
        cli('close', remaining)
        saved = frame(inspect(remaining))
        expected.append(open_note('n'))
        assert not inspect(remaining)['visible'] and frame(inspect(remaining)) == saved
        check('unrelated hidden note remains hidden and keeps geometry across later opens')
        log['outcome'] = 'passed'
    except BaseException as error:
        log['outcome'] = 'failed'
        log['error'] = repr(error)
        raise
    finally:
        for note in ids:
            cli('close', note)
        move(original_pointer['x'], original_pointer['y'])
        subprocess.run(['osascript', '-e', f'tell application "System Events" to set frontmost of (first application process whose unix id is {original_front}) to true'], capture_output=True)
        log['front_before'] = original_front
        log['front_after_cleanup'] = front()
        (folder / 'live.json').write_text(json.dumps(log, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
