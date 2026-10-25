#!/usr/bin/env python3
"""Exercise a real running native app. Leaves the note hidden on success."""
import json
import os
from pathlib import Path
import socket
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
BIN = str(ROOT / 'scripts/agent-gibbon')

def call(*args, ok=True):
    result = subprocess.run([BIN, *args], capture_output=True, text=True, timeout=25)
    data = json.loads(result.stdout)
    assert data['ok'] is ok, data
    assert (result.returncode == 0) is ok, result
    return data

def stable(before, after):
    for key in ('position', 'size', 'focused', 'visible', 'always_on_top'):
        assert before[key] == after[key], (key, before[key], after[key])

call('bad-command', ok=False)
call('close', '--title', 'invalid', ok=False)
opened = call('open', '--title', '실환경 검증', '--file', str(ROOT / 'examples/progress.md'), '--status', '검증 중')
assert opened['visible']
before = call('inspect')
for index in range(5):
    updated = call('update', '--status', f'갱신 {index}')
    stable(before, updated)
assert updated['note']['title'] == '실환경 검증'
assert '☑' in updated['note']['html'] and '☐' in updated['note']['html']
closed = call('close')
assert not closed['visible']
updated = call('update', '--markdown', '# 한글\n- [x] 완료\n<script>alert(1)</script>\n![추적](https://example.com/a)')
stable(closed, updated)
assert not updated['visible'] and '<script' not in updated['note']['html'] and '<img' not in updated['note']['html']
reopened = call('open')
assert reopened['visible'] and reopened['position'] == closed['position'] and reopened['size'] == closed['size']
assert reopened['note']['html'] == updated['note']['html']
# Malformed socket clients must not kill the listener.
with socket.socket(socket.AF_UNIX) as stream:
    stream.connect(f'/tmp/floatnote-{os.getuid()}/app.sock')
    stream.sendall(b'{bad json')
    stream.shutdown(socket.SHUT_WR)
    response = bytearray()
    while chunk := stream.recv(4096):
        response.extend(chunk)
    assert json.loads(response)['ok'] is False
assert call('inspect')['visible']
call('close')
print('PASS: JSON errors, native open/update/close/reopen, repeated updates, geometry/focus/hidden preservation, Korean tasks, unsafe markup, malformed IPC')
