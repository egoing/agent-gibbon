#!/usr/bin/env python3
"""Two built worktrees + existing default instance. Never stops pre-existing apps.
Usage: python3 scripts/isolation-smoke.py --peer /absolute/other-worktree
"""
import argparse
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
import tempfile
import uuid

parser = argparse.ArgumentParser()
parser.add_argument('--peer', required=True, type=Path)
args = parser.parse_args()
ROOT = Path(__file__).resolve().parents[1]
PEER = args.peer.resolve()
assert ROOT != PEER


def info(root):
    return json.loads(subprocess.check_output(['node', str(root / 'scripts/dev.mjs'), 'info'], text=True))


def raw(path, request):
    with socket.socket(socket.AF_UNIX) as stream:
        stream.settimeout(10)
        stream.connect(str(path))
        stream.sendall(json.dumps(request).encode())
        stream.shutdown(socket.SHUT_WR)
        chunks = []
        while chunk := stream.recv(65536):
            chunks.append(chunk)
        return json.loads(b''.join(chunks))


def live(path):
    try:
        return raw(path, {'action': 'list'})['ok']
    except (OSError, KeyError, json.JSONDecodeError):
        return False


def call(root, *args, ok=True, cwd="/"):
    result = subprocess.run([str(root / 'scripts/agent-gibbon-dev'), *args], cwd=cwd, capture_output=True, text=True, timeout=30)
    data = json.loads(result.stdout)
    assert data['ok'] is ok and (result.returncode == 0) is ok, (result.stderr, data)
    return data


def pid_for(binary):
    # Cargo runs target/debug/agent-gibbon with a relative argv[0]. Resolve by the
    # executable vnode, not a process-name match that could select another app.
    result = subprocess.run(['lsof', '-t', binary], capture_output=True, text=True)
    pids = sorted({int(pid) for pid in result.stdout.split()})
    assert len(pids) == 1, pids
    return pids[0]


def wait_for(path, expected=True):
    until = time.monotonic() + 120
    while live(path) != expected:
        assert time.monotonic() < until, f'timeout: {path}'
        time.sleep(.2)


def state_only(state):
    return {key: state[key] for key in ('note', 'position', 'size', 'visible', 'focused', 'always_on_top')}


ia, ib = info(ROOT), info(PEER)
assert ia['id'] != ib['id']
sa, sb = [Path(f'/tmp/floatnote-{os.getuid()}-{i["id"]}/app.sock') for i in (ia, ib)]
# Refuse to assume ownership of any existing development process.
assert not live(sa) and not live(sb), '개발 앱이 이미 실행 중입니다. 별도의 검증 워크트리를 사용하세요.'
default = Path(f'/tmp/floatnote-{os.getuid()}/app.sock')
assert live(default), '기존 설치 앱이 실행 중이어야 합니다.'
installed = raw(default, {'action': 'list'})
baseline = {w['id']: state_only(raw(default, {'action': 'inspect', 'id': w['id']})) for w in installed['windows']}

def check_installed():
    now = raw(default, {'action': 'list'})
    assert now == installed, '설치 앱 메모 목록 변경'
    for key, before in baseline.items():
        assert state_only(raw(default, {'action': 'inspect', 'id': key})) == before, f'설치 앱 상태 변경: {key}'

probe_dir = tempfile.TemporaryDirectory(prefix='floatnote-focus-')
probe = str(Path(probe_dir.name) / 'frontmost')
subprocess.run(['swiftc', str(ROOT / 'scripts/frontmost.swift'), '-o', probe], check=True)
def frontmost():
    return int(subprocess.check_output([probe], text=True))
original_frontmost = frontmost()
owned = set()
runner = None
note = 'isolation-' + uuid.uuid4().hex[:8]
try:
    # CLI auto-launch: same instance even when invoked outside its worktree.
    a = call(ROOT, 'open', '--id', note, '--markdown', 'worktree A')
    owned.add(pid_for(ia['binary']))
    assert frontmost() == original_frontmost, 'CLI auto-launch changed frontmost app'
    assert a['instance'] == ia['id'] and a['dev_label'].startswith('DEV ')
    # npm dev must own B's socket, not silently reuse A/default or a release binary.
    log_path = Path('/tmp') / f'{note}-npm-dev.log'
    with log_path.open('w') as log:
        runner = subprocess.Popen(['npm', 'run', 'dev', '--', '--no-watch'], cwd=PEER, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    wait_for(sb)
    owned.add(pid_for(ib['binary']))
    assert frontmost() == original_frontmost, 'npm dev changed frontmost app'
    b = call(PEER, 'open', '--id', note, '--markdown', 'worktree B')
    assert frontmost() == original_frontmost, 'new webview changed frontmost app'
    assert b['instance'] == ib['id'] and a['dev_label'] != b['dev_label']
    for root in (ROOT, PEER):
        assert call(root, '--serve')['already_running'] is True
    assert call(ROOT, 'inspect', '--id', note)['note']['markdown'] == 'worktree A'
    assert call(PEER, 'inspect', '--id', note)['note']['markdown'] == 'worktree B'
    # Hiding cancels entrance animation; metadata changes must preserve exact geometry.
    for root in (ROOT, PEER):
        call(root, 'close', '--id', note)
    b_before = state_only(call(PEER, 'inspect', '--id', note))
    a_before = call(ROOT, 'inspect', '--id', note)
    a_after = call(ROOT, 'update', '--id', note, '--markdown', 'A changed while hidden')
    for key in ('visible', 'position', 'size', 'focused', 'always_on_top'):
        assert a_before[key] == a_after[key], key
    assert frontmost() == original_frontmost, 'update changed frontmost app'
    assert a_after['visible'] is False and a_after['focused'] is False
    assert state_only(call(PEER, 'inspect', '--id', note)) == b_before
    # Each worktree edits its own file; shared source files are intentionally not isolated.
    for root, content in ((ROOT, 'A file'), (PEER, 'B file')):
        source = root / f'.{note}.md'
        source.write_text(content)
        try:
            state = call(root, 'update', '--id', note, '--file', source.name, cwd=root)
            assert state['note']['markdown'] == content
            assert state['note']['source_path'] == str(source)
        finally:
            source.unlink()
    check_installed()
    a_pid = pid_for(ia['binary'])
    os.kill(a_pid, signal.SIGTERM)
    owned.remove(a_pid)
    wait_for(sa, False)
    assert call(PEER, 'inspect', '--id', note)['note']['markdown'] == 'B file'
    check_installed()
    # Restart reclaims a stale socket/lock without disturbing B or installed notes.
    a = call(ROOT, 'open', '--id', note, '--markdown', 'A restarted')
    owned.add(pid_for(ia['binary']))
    assert a['note']['markdown'] == 'A restarted'
    assert call(PEER, 'inspect', '--id', note)['note']['markdown'] == 'B file'
    check_installed()
    print(json.dumps({'ok': True, 'checks': ['CLI auto-launch', 'npm dev same namespace', 'same-ID content isolation', 'independent duplicate locks', 'hidden update preserves geometry/focus', 'separate source files', 'termination/restart isolation', 'installed app unchanged', 'native frontmost application preserved'], 'instances': [ia['id'], ib['id']], 'npm_log': str(log_path)}, ensure_ascii=False))
finally:
    for pid in owned:
        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    if runner is not None:
        try:
            os.killpg(runner.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        runner.wait(timeout=15)
    probe_dir.cleanup()
