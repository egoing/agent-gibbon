#!/usr/bin/env python3
"""Verify scroll consumption against a separate native app, in this worktree.

Requires a stopped debug instance and a free modifier-input lease. Never stops
other apps. Restores the instance setting, pointer and original foreground app.
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

assert sys.platform == 'darwin'
ROOT = Path(__file__).resolve().parents[1]
INFO = json.loads(subprocess.check_output(['node', str(ROOT / 'scripts/dev.mjs'), 'info'], text=True))
BIN = str(ROOT / 'scripts/agent-gibbon-dev')
EXPECTED = INFO['binary'] + ' --serve'
SETTING = Path.home() / 'Library/Application Support/org.egoing.floatnote/appearance' / INFO['id'] / 'opacity.json'
OUTPUT = ROOT / 'docs/verification/hover-opacity/scroll-evidence.json'
NOTE = 'opacity-scroll-' + uuid.uuid4().hex[:8]
CHORD = (1 << 18) | (1 << 19) | (1 << 20)


def processes():
    lines = subprocess.check_output(['ps', '-axo', 'pid=,command='], text=True).splitlines()
    return [int(p[0]) for line in lines if len(p := line.strip().split(None, 1)) == 2 and p[1] == EXPECTED]


assert Path(INFO['binary']).is_file(), 'Run pnpm run dev:build first'
assert not processes(), 'Existing worktree app must be stopped by its owner first'
saved = SETTING.read_bytes() if SETTING.exists() else None
evidence = {'instance': INFO, 'note_id': NOTE, 'samples': [], 'passed': False}
pid = None
receiver = None
pointer = None


def cli(action, *args):
    result = subprocess.run([BIN, action, '--id', NOTE, *args], text=True, capture_output=True, timeout=25)
    assert result.returncode == 0, result.stdout + result.stderr
    data = json.loads(result.stdout)
    assert data['ok'] and data['instance'] == INFO['id'] and data['id'] == NOTE, data
    return data


with tempfile.TemporaryDirectory(prefix='gibbon-scroll-probe-') as folder:
    folder = Path(folder)
    helper = folder / 'probe'
    source = helper.with_suffix('.swift')
    state = folder / 'state.json'
    source.write_text(r'''
import AppKit
import CoreGraphics
let a = CommandLine.arguments
if a[1] == "pointer" {
 let p = CGEvent(source:nil)!.location
 print("\(p.x) \(p.y)")
} else if a[1] == "front" {
 print(NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0)
} else if a[1] == "restore" {
 NSRunningApplication(processIdentifier:Int32(a[2])!)?.activate(options:[])
} else if a[1] == "move" {
 let p = CGPoint(x:Double(a[2])!, y:Double(a[3])!)
 CGWarpMouseCursorPosition(p)
 CGEvent(mouseEventSource:nil, mouseType:.mouseMoved, mouseCursorPosition:p, mouseButton:.left)?.post(tap:.cghidEventTap)
} else if a[1] == "send" {
 let precise = a[5] == "1"
 let e = CGEvent(scrollWheelEvent2Source:nil, units:precise ? .pixel : .line, wheelCount:2, wheel1:Int32(a[2])!, wheel2:Int32(a[3])!, wheel3:0)!
 e.flags = CGEventFlags(rawValue:UInt64(a[4])!)
 let point = CGEvent(source:nil)!.location
 e.location = point
 e.setIntegerValueField(.scrollWheelEventScrollPhase, value:Int64(a[6])!)
 e.setIntegerValueField(.scrollWheelEventMomentumPhase, value:Int64(a[7])!)
 e.post(tap:.cghidEventTap)
} else {
 let app = NSApplication.shared
 app.setActivationPolicy(.accessory)
 class Probe: NSScrollView {
  var count = 0
  override func scrollWheel(with event:NSEvent) {
   count += 1
   super.scrollWheel(with:event)
  }
 }
 let window = NSPanel(contentRect:NSRect(x:100,y:250,width:500,height:400), styleMask:[.titled,.nonactivatingPanel], backing:.buffered, defer:false)
 window.title = "Agent Gibbon separate scroll receiver"
 window.isReleasedWhenClosed = false
 let probe = Probe(frame:NSRect(x:0,y:0,width:500,height:400))
 probe.hasVerticalScroller = true
 probe.hasHorizontalScroller = true
 probe.documentView = NSView(frame:NSRect(x:0,y:0,width:3000,height:6000))
 window.contentView = probe
 window.orderFrontRegardless()
 probe.contentView.scroll(to:NSPoint(x:1000,y:2000))
 probe.reflectScrolledClipView(probe.contentView)
 let path = URL(fileURLWithPath:a[2])
 let timer = Timer.scheduledTimer(withTimeInterval:0.03,repeats:true) { _ in
  let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly,kCGNullWindowID) as! [[String:Any]]
  let target = windows.first { ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == ProcessInfo.processInfo.processIdentifier && ($0[kCGWindowName as String] as? String) == window.title }
  let data:[String:Any] = ["pid":ProcessInfo.processInfo.processIdentifier,"count":probe.count,"x":probe.contentView.bounds.origin.x,"y":probe.contentView.bounds.origin.y,"window":target ?? [:]]
  try! JSONSerialization.data(withJSONObject:data).write(to:path,options:.atomic)
 }
 withExtendedLifetime(timer) { app.run() }
}
''')
    subprocess.run(['swiftc', str(source), '-o', str(helper)], check=True)

    def native(*args):
        return subprocess.check_output([str(helper), *map(str, args)], text=True).strip()

    def read_state():
        return json.loads(state.read_text())

    def sample(stage):
        data = read_state()
        note = cli('inspect')
        evidence['samples'].append({'stage': stage, 'receiver': data, 'opacity': note['global_opacity'], 'stash': note['stash']})
        assert native('front') == front, 'Unexpected foreground change'
        return data, note

    def send(y, x=0, flags=0, precise=0, phase=0, momentum=0):
        native('send', y, x, flags, precise, phase, momentum)
        time.sleep(.22)

    def blocked(stage, before):
        after, note = sample(stage)
        assert (after['count'], after['x'], after['y']) == (before['count'], before['x'], before['y']), (stage, before, after)
        assert not note['stash']['active'], 'Wheel must not toggle stash'
        return after, note

    try:
        pointer = native('pointer').split()
        front = native('front')
        SETTING.parent.mkdir(parents=True, exist_ok=True)
        SETTING.write_bytes(b'{"percent":35}')
        note = cli('open', '--title', 'Scroll consumption verification', '--markdown', f'# {NOTE}\n\nSeparate native scroll receiver test')
        assert NOTE in note['note']['markdown']
        pid, = processes()
        evidence['pid'] = pid
        files = subprocess.check_output(['/usr/sbin/lsof', '-a', '-p', str(pid), '-d', 'txt', '-Fn'], text=True).splitlines()
        assert 'n' + INFO['binary'] in files
        evidence['verified_executable'] = INFO['binary']
        note = cli('inspect')
        assert note['stash']['shortcut'] == 'Ctrl+Alt+Cmd', 'Input lease unavailable or accessibility permission missing: ' + note['stash']['shortcut']
        receiver = subprocess.Popen([str(helper), 'receive', str(state)])
        for _ in range(100):
            if state.exists() and read_state().get('window'):
                break
            time.sleep(.05)
        rect = read_state()['window']['kCGWindowBounds']
        native('move', rect['X'] + rect['Width']/2, rect['Y'] + rect['Height']/2)
        time.sleep(.5)
        baseline, _ = sample('baseline')
        send(-3)
        normal, _ = sample('normal-scroll-delivered')
        assert normal['count'] > baseline['count'] and normal['y'] != baseline['y']
        send(1, flags=CHORD)
        current, note = blocked('chord-scroll-consumed', normal)
        assert note['global_opacity'] == .4, note['global_opacity']
        send(1, flags=CHORD | (1 << 17))
        shifted, note = sample('shift-scroll-delivered')
        assert shifted['count'] > current['count'] and note['global_opacity'] == .4
        send(0, x=3, flags=CHORD)
        horizontal, note = sample('horizontal-scroll-delivered')
        assert horizontal['count'] > shifted['count'] and note['global_opacity'] == .4
        send(5, flags=CHORD, precise=1, phase=1)
        current, note = blocked('partial-precise-consumed', horizontal)
        assert note['global_opacity'] == .4
        send(15, flags=CHORD, precise=1, phase=2)
        current, note = blocked('precise-step-consumed', current)
        assert note['global_opacity'] == .45
        send(0, precise=1, phase=4)
        current, _ = blocked('gesture-end-after-release-consumed', current)
        send(10, precise=1, momentum=1)
        current, note = blocked('captured-momentum-consumed', current)
        assert note['global_opacity'] == .45
        send(0, precise=1, momentum=3)
        current, _ = blocked('momentum-end-consumed', current)
        for _ in range(15):
            send(1, flags=CHORD)
        current, note = blocked('upper-limit-still-consumed', current)
        assert note['global_opacity'] == 1.
        send(-3)
        resumed, note = sample('normal-scroll-resumed')
        assert resumed['count'] > current['count'] and resumed['y'] != current['y']
        assert note['global_opacity'] == 1.
        evidence['passed'] = True
        print(f'PASS: separate app normal/Shift/horizontal delivery, chord/fraction/limit consumption, trackpad momentum tail; PID={pid}', flush=True)
    finally:
        if receiver:
            receiver.terminate()
            receiver.wait(timeout=5)
        if pid is None:
            found = processes()
            if len(found) == 1:
                pid = found[0]
        if pid in processes():
            cli('close')
            os.kill(pid, signal.SIGTERM)
            for _ in range(50):
                if pid not in processes():
                    break
                time.sleep(.1)
        if pointer:
            native('move', *pointer)
        if native('front') != front:
            native('restore', front)
        if saved is None:
            SETTING.unlink(missing_ok=True)
        else:
            SETTING.write_bytes(saved)
        evidence['test_server_stopped'] = not processes()
        evidence['setting_restored'] = (SETTING.read_bytes() if SETTING.exists() else None) == saved
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + '\n')
