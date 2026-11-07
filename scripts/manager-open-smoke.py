#!/usr/bin/env python3
"""macOS #84: one real tray selection must show AND activate the manager.

Run on a freshly started, worker-owned dev instance with no manager yet. Requires
Accessibility and event-posting permission. Never stops apps. Restores the prior
frontmost app and pointer; hides only the uniquely named test notes it creates.
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
    return subprocess.check_output(list(map(str, args)), text=True, timeout=30).strip()


def apple(script):
    return run('osascript', '-e', script)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pid', type=int, required=True)
    parser.add_argument('--report-dir', type=Path, required=True)
    args = parser.parse_args()
    out = args.report_dir.resolve()
    out.mkdir(parents=True, exist_ok=True)
    info = json.loads(run('node', ROOT / 'scripts/dev.mjs', 'info'))
    assert run('ps', '-p', args.pid, '-o', 'comm=') == info['binary'], 'foreign PID'
    for source, name in [('manager-open-native.swift', 'probe'), ('tray-menu-ax.swift', 'menu'),
                         ('macos-input.swift', 'input'), ('entrance-pointer.swift', 'pointer')]:
        subprocess.run(['swiftc', str(ROOT / 'scripts' / source), '-o', str(out / name)], check=True)

    def cli(*argv):
        result = json.loads(run(CLI, *argv))
        assert result['ok'], result
        if argv[0] == 'inspect':
            assert result['instance'] == info['id'] and result['id'] == argv[2], result
        return result

    def probe():
        return json.loads(run(out / 'probe', args.pid))

    def manager(state):
        return next((w for w in state['windows'] if '메모 관리' in w['title']), None)

    before = probe()
    assert manager(before) is None, 'restart only your own dev PID before this test'
    original_front = before['frontmost']
    original_pointer = json.loads(run(out / 'pointer'))
    prefix = f'tell application "System Events" to tell (first application process whose unix id is {args.pid}) to '
    proof = 'manager84-' + uuid.uuid4().hex[:8]
    hidden = proof + '-hidden'
    report = {'origin': info, 'pid': args.pid, 'cases': [], 'failures': []}
    try:
        cli('open', '--id', proof, '--title', proof, '--markdown', '# ' + proof)
        cli('open', '--id', hidden, '--title', hidden, '--markdown', '# ' + hidden)
        cli('close', '--id', hidden)
        time.sleep(2)  # Finish memo entrance before geometry preservation checks.
        report['proof'] = cli('inspect', '--id', proof)
        assert proof in report['proof']['note']['markdown']
        assert any(proof in w.get('kCGWindowName', '') for w in probe()['onscreen']), 'native title missing'
        preserved = {id: cli('inspect', '--id', id) for id in [proof, hidden]}
        for name in ['first-create', 'existing-background', 'hidden-restore', 'minimized-restore']:
            if name == 'hidden-restore':
                apple(prefix + 'click button 1 of (first window whose name contains "메모 관리")')
                assert manager(probe()) is None, 'close did not hide manager'
            elif name == 'minimized-restore':
                apple(prefix + 'set value of attribute "AXMinimized" of (first window whose name contains "메모 관리") to true')
                assert manager(probe())['minimized'], 'minimize preparation failed'
            apple('tell application "Finder" to activate')
            time.sleep(.2)
            pre = probe()
            assert pre['frontmost'] != args.pid, 'background preparation failed'
            x, y, width, height = map(float, apple(prefix + 'get {position, size} of menu bar item 1 of menu bar 2').split(','))
            # Deliver Enter/refresh before mouse-down, as an actual pointer
            # approach does (same sequence as tray-menu-polish-smoke.py).
            run(out / 'pointer', 'move', x - 100, y + 100)
            run(out / 'pointer', 'move', x + width / 2, y + height / 2)
            time.sleep(.15)
            run(out / 'input', 'click', x + width / 2, y + height / 2)
            menu = json.loads(run(out / 'menu', args.pid))
            assert menu[0]['title'] == info['name'] and menu[1]['title'] == '메모 관리', menu
            selected_at = time.monotonic()
            run(out / 'menu', args.pid, '메모 관리')  # Exactly one menu event.
            samples = []
            # Observe asynchronous unminimization without a second focus request.
            for delay in [.05, .2, .5, 1]:
                time.sleep(delay)
                samples.append({'elapsed': time.monotonic() - selected_at, 'state': probe()})
            state = samples[-1]['state']
            window = manager(state)
            onscreen = any('메모 관리' in w.get('kCGWindowName', '') for w in state['onscreen'])
            passed = bool(window and not window['minimized'] and window['main']
                          and onscreen and state['frontmost'] == args.pid)
            report['cases'].append({'name': name, 'before': pre, 'samples': samples, 'passed': passed})
            if not passed:
                report['failures'].append(name)
            print(('PASS' if passed else 'FAIL') + ': ' + name, flush=True)
            for id, previous in preserved.items():
                current = cli('inspect', '--id', id)
                for key in ['position', 'size', 'visible', 'always_on_top']:
                    assert previous[key] == current[key], (name, id, key, previous[key], current[key])
        report['memo_state_preserved'] = True
    finally:
        (out / 'report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2))
        for id in [proof, hidden]:
            cli('close', '--id', id)
        run(out / 'pointer', 'move', original_pointer['x'], original_pointer['y'])
        apple(f'tell application "System Events" to set frontmost of (first application process whose unix id is {original_front}) to true')
    assert not report['failures'], 'single selection did not activate manager: ' + ', '.join(report['failures'])


if __name__ == '__main__':
    main()
