#!/usr/bin/env python3
"""Native macOS feedback verification against an explicitly isolated instance.
FLOATNOTE_TEST_BIN and FLOATNOTE_TEST_PROCESS are required. Uses only unique notes,
restores pointer/front app, and hides test notes. Never stops an existing app.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import uuid

BIN = os.environ['FLOATNOTE_TEST_BIN']
PROCESS = os.environ['FLOATNOTE_TEST_PROCESS']

def apple(code):
    return subprocess.check_output(['osascript', '-e', code], text=True, timeout=15).strip()

def front():
    return apple('tell application "System Events" to get name of first application process whose frontmost is true')

def cli(action, note_id, *args):
    result = subprocess.run([BIN, action, '--id', note_id, *args], capture_output=True, text=True, timeout=25)
    assert result.returncode == 0, result.stderr + result.stdout
    data = json.loads(result.stdout)
    assert data['ok'], data
    return data

def stable(a, b):
    for key in ('position', 'size', 'focused', 'always_on_top', 'visible'):
        assert a[key] == b[key], (key, a[key], b[key])

def passed(message):
    print('PASS: ' + message, flush=True)

original_front = front()
prefix = 'feedback-' + uuid.uuid4().hex[:8]
a, b = prefix + '-a', prefix + '-b'
created = []
with tempfile.TemporaryDirectory(prefix='floatnote-feedback-native-') as folder:
    folder = Path(folder)
    swift = folder / 'native.swift'
    swift.write_text(r'''
import Foundation
import CoreGraphics
let args = CommandLine.arguments
if args[1] == "pointer" {
 let p = CGEvent(source:nil)!.location
 print("\(p.x) \(p.y)")
} else if args[1] == "move" {
 CGEvent(mouseEventSource:nil, mouseType:.mouseMoved, mouseCursorPosition:CGPoint(x:Double(args[2])!, y:Double(args[3])!), mouseButton:.left)!.post(tap:.cghidEventTap)
} else {
 let all = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as! [[String:Any]]
 let filtered = all.enumerated().compactMap { index, w -> [String:Any]? in
  let title = w[kCGWindowName as String] as? String ?? ""
  let pid = (w[kCGWindowOwnerPID as String] as? NSNumber)?.intValue ?? 0
  guard args.count > 3 ? pid == Int(args[3])! : title.contains(args[2]) else { return nil }
  return ["title":title, "order":index, "alpha":w[kCGWindowAlpha as String]!, "id":w[kCGWindowNumber as String]!, "bounds":w[kCGWindowBounds as String]!]
 }
 print(String(data:try! JSONSerialization.data(withJSONObject:filtered), encoding:.utf8)!)
}
''')
    helper = folder / 'native'
    subprocess.run(['swiftc', str(swift), '-o', str(helper)], check=True, capture_output=True)
    def native(*args):
        return subprocess.check_output([str(helper), *args], text=True).strip()
    pointer = native('pointer').split()
    def windows():
        args = ['windows', prefix]
        if os.environ.get('FLOATNOTE_TEST_PID'):
            args.append(os.environ['FLOATNOTE_TEST_PID'])
        raw = json.loads(native(*args))  # Capture before inspect can synchronize alpha.
        result = {}
        for note_id in (a, b):
            state = cli('inspect', note_id)
            for w in raw:
                rect = w['bounds']
                scale = state['size']['width'] / rect['Width']
                if abs(state['position']['x'] / scale - rect['X']) < 2 and abs(state['position']['y'] / scale - rect['Y']) < 2:
                    result[note_id] = w
        return result
    def root(note_id):
        return 'group "Floatnote" of UI element 1 of scroll area 1 of group 1 of group 1 of window ' + json.dumps('Floatnote — ' + note_id)
    def ui(code):
        return apple(f'tell application "System Events" to tell process {json.dumps(PROCESS)} to {code}')
    try:
        body = '# 변경 확인\n\n유지하는 문단\n\n이전 문단'
        cli('open', a, '--markdown', body); created.append(a)
        cli('open', b, '--markdown', '다른 노트'); created.append(b)
        native('move', '5', '40'); time.sleep(.6)
        before = cli('inspect', a)
        assert not before['feedback_active'] and abs(windows()[a]['alpha'] - .5) < .01
        assert windows()[b]['order'] < windows()[a]['order']
        active_app = front()
        changed = body.replace('이전 문단', '바뀐 문단')
        started = time.monotonic(); result = cli('update', a, '--markdown', changed)
        stable(before, result)
        assert result['feedback_active'] and result['opacity'] == 1
        native_state = windows()
        assert native_state[a]['order'] < native_state[b]['order'], native_state
        assert native_state[a]['alpha'] == 1 and native_state[b]['alpha'] == .5
        assert front() == active_app
        assert '<mark>바뀐 문단</mark>' in result['note']['change_html']
        assert '<mark>유지하는' not in result['note']['change_html']
        assert '<mark>' not in result['note']['html'] and result['note']['markdown'] == changed
        time.sleep(.45)
        if os.environ.get('FLOATNOTE_TEST_SCREENSHOT'):
            subprocess.run(['screencapture', '-x', '-l', str(native_state[a]['id']), os.environ['FLOATNOTE_TEST_SCREENSHOT']], check=True)
        passed('native order/alpha, focus/position/pin preservation, isolated safe rendering payload and clean source')
        time.sleep(max(0, 2.0 - (time.monotonic() - started)))
        cli('update', a, '--markdown', changed + '!')
        time.sleep(1.2)
        assert windows()[a]['alpha'] == 1, 'old deadline ended newer feedback'
        cli('update', a, '--status', '메타데이터')
        cli('update', a, '--markdown', changed + '!')
        time.sleep(2.0)
        assert abs(windows()[a]['alpha'] - .5) < .01, 'same body/metadata restarted feedback or native expiry failed'
        expired = cli('inspect', a)
        assert not expired['feedback_active'] and expired['highlight_revision'] is not None
        passed('last actual change + 3 seconds, stale deadline immunity, metadata/same-body no restart')
        rect = windows()[a]['bounds']
        native('move', str(rect['X'] + rect['Width'] / 2), str(rect['Y'] + rect['Height'] / 2))
        time.sleep(.6)
        actual_pointer = [float(v) for v in native('pointer').split()]
        expected_pointer = [rect['X'] + rect['Width'] / 2, rect['Y'] + rect['Height'] / 2]
        if all(abs(actual - expected) < 2 for actual, expected in zip(actual_pointer, expected_pointer)):
            assert cli('inspect', a)['highlight_revision'] is None, 'pointer contact did not acknowledge highlight'
            native('move', '5', '40'); time.sleep(.2)
            cli('update', a, '--markdown', changed + ' 다시 변경')
            time.sleep(.45)
            rect = windows()[a]['bounds']
            native('move', str(rect['X'] + rect['Width'] / 2), str(rect['Y'] + rect['Height'] / 2))
            time.sleep(.4)
            native('move', '5', '40'); time.sleep(.2)
            acknowledged = cli('inspect', a)
            assert acknowledged['highlight_revision'] is None and acknowledged['feedback_active'] and acknowledged['opacity'] == 1
            time.sleep(3.0)
            passed('highlight survives opacity expiry, real pointer contact acknowledges it, opacity deadline remains independent')
        else:
            print('SKIP: pointer acknowledgement; CGEvent movement was not delivered (requested=' + str(expected_pointer) + ', actual=' + str(actual_pointer) + ')', flush=True)
        if os.environ.get('FLOATNOTE_TEST_UI') == '1':
            # Unpinned notes still move forward without changing pin or focus.
            ui('click button "상단 고정" of ' + root(a)); time.sleep(.25)
            before = cli('inspect', a)
            assert not before['always_on_top']
            result = cli('update', a, '--markdown', changed)
            stable(before, result)
            assert front() == active_app
            passed('unpinned note preserves pin, geometry and active application')
        else:
            print('SKIP: unpinned button interaction (Accessibility permission unavailable)', flush=True)
        cli('close', a)
        result = cli('update', a, '--markdown', '숨긴 갱신')
        assert not result['visible'] and not result['feedback_active'] and a not in windows()
        cli('open', a)
        reopened = cli('inspect', a)
        assert not reopened['feedback_active'] and reopened['highlight_revision'] is not None
        passed('hide clears opacity only, hidden update stays hidden, reopen preserves highlight')
        result = cli('update', a, '--markdown', '')
        assert result['feedback_active'] and result['note']['removed'] and result['note']['html'] == ''
        if os.environ.get('FLOATNOTE_TEST_UI') == '1':
            time.sleep(.45)
            contents = ui('get entire contents of window ' + json.dumps('Floatnote — ' + a))
            assert '이전 내용 삭제됨' in contents, contents
            time.sleep(3.1)
            contents = ui('get entire contents of window ' + json.dumps('Floatnote — ' + a))
            assert '이전 내용 삭제됨' in contents
            passed('deletion-only empty body notice remains after opacity expiry in real WebView')
        else:
            time.sleep(3.2)
            deletion = cli('inspect', a)
            assert not deletion['feedback_active'] and deletion['highlight_revision'] is not None
            passed('empty-body deletion highlight remains after opacity expiry through real CLI')
            print('SKIP: visual highlight/deletion notice, editor and copy UI (Accessibility/Screen Recording unavailable)', flush=True)
    finally:
        for note_id in created:
            cli('close', note_id)
        native('move', *pointer)
        apple('tell application "System Events" to set frontmost of process ' + json.dumps(original_front) + ' to true')
