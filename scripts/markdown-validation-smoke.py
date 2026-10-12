#!/usr/bin/env python3
"""Reject malformed tasks without mutating a note, then accept corrected input."""
import json
from pathlib import Path
import socket
import subprocess
import os
import uuid

ROOT = Path(__file__).resolve().parents[1]
BIN = str(ROOT / 'scripts/floatnote')
note_id = 'validation-' + uuid.uuid4().hex[:8]

def cli(*args):
    result = subprocess.run([BIN, *args], text=True, capture_output=True, check=True)
    return json.loads(result.stdout)

try:
    cli('open', '--id', note_id, '--markdown', '- [ ] 원래 항목')
    before = cli('inspect', '--id', note_id)['note']
    bad = subprocess.run([BIN, 'update', '--id', note_id, '--title', '변경 금지',
                          '--markdown', '# 제목\n[x] 잘못된 항목'], text=True, capture_output=True)
    error = json.loads(bad.stdout)
    assert bad.returncode == 1 and not error['ok']
    assert 'MARKDOWN_TASK_MARKER' in error['error'] and '2행' in error['error']
    assert '- [x]' in error['error']
    # Bypass CLI validation to verify that the server enforces the same rule.
    request = {'action': 'update', 'id': note_id, 'color': '#abcdef', 'markdown': '[ ] 잘못된 항목'}
    with socket.socket(socket.AF_UNIX) as client:
        client.settimeout(10)
        client.connect(f'/tmp/floatnote-{os.getuid()}/app.sock')
        client.sendall(json.dumps(request).encode() + b'\n')
        client.shutdown(socket.SHUT_WR)
        raw = client.makefile('rb').readline()
    rejected = json.loads(raw)
    assert not rejected['ok'] and 'MARKDOWN_TASK_MARKER' in rejected['error']
    assert '1행' in rejected['error']
    assert cli('inspect', '--id', note_id)['note'] == before
    fixed = cli('update', '--id', note_id, '--markdown', '- [x] 수정된 항목')
    assert '☑ 수정된 항목' in fixed['note']['html']
    cli('update', '--id', note_id, '--markdown', '```markdown\n[ ] 코드 예제\n```')
    print('PASS: CLI error/line/correction, raw IPC rejection, atomic state, corrected retry and code exemption')
finally:
    cli('close', '--id', note_id)
