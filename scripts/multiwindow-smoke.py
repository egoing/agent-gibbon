#!/usr/bin/env python3
"""Real native CLI regression. Creates isolated notes and leaves them hidden."""
import json
import os
from pathlib import Path
import socket
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[1]
BIN = str(ROOT / 'scripts/floatnote')


def call(*args, ok=True):
    result = subprocess.run([BIN, *args], capture_output=True, text=True, timeout=25)
    data = json.loads(result.stdout)
    assert data['ok'] is ok, data
    assert (result.returncode == 0) is ok, result
    return data


def raw(request):
    with socket.socket(socket.AF_UNIX) as stream:
        stream.settimeout(10)
        stream.connect(f'/tmp/floatnote-{os.getuid()}/app.sock')
        stream.sendall(json.dumps(request).encode())
        stream.shutdown(socket.SHUT_WR)
        data = bytearray()
        while chunk := stream.recv(4096):
            data.extend(chunk)
        return json.loads(data)


def stable(before, after):
    for key in ('position', 'size', 'focused', 'visible', 'always_on_top'):
        assert before[key] == after[key], (key, before, after)


def snapshot(note_id):
    return call('inspect', '--id', note_id)


prefix = 'smoke-' + uuid.uuid4().hex[:8]
a, b, missing = prefix + '-a', prefix + '-b', prefix + '-missing'
default = call('inspect')
created = []
try:
    for command in ('inspect', 'update', 'close'):
        call(command, '--id', missing, ok=False)
    call('open', '--id', missing, '--color', 'red', ok=False)
    first = call('open', '--id', a, '--title', '창 A', '--color', '#FFE8A3', '--markdown', '- [x] 완료')
    created.append(a)
    second = call('open', '--id', b, '--title', '창 B', '--color', '#18304A', '--markdown', '- [ ] 대기')
    created.append(b)
    assert first['note']['color'] == '#ffe8a3'
    assert second['note']['color'] == '#18304a'
    assert first['visible'] and second['visible']
    assert snapshot(a) == first
    stable(default, call('inspect'))
    assert default['note'] == call('inspect')['note']
    for index in range(5):
        updated = call('update', '--id', a, '--status', str(index))
        stable(first, updated)
        assert updated['note']['color'] == first['note']['color']
        assert snapshot(b) == second
    before = snapshot(a)
    assert not raw({'action': 'update', 'id': a, 'title': 'must not change', 'color': 'bad'})['ok']
    assert not raw({'action': 'open', 'id': '../invalid'})['ok']
    assert snapshot(a) == before
    hidden = call('close', '--id', a)
    assert not hidden['visible']
    updated = call('update', '--id', a, '--color', '#112233', '--title', '숨긴 창')
    stable(hidden, updated)
    assert updated['note']['color'] == '#112233'
    assert snapshot(b) == second
    listing = call('list')['windows']
    ids = [entry['id'] for entry in listing]
    assert ids == sorted(ids) and len(ids) == len(set(ids))
    assert a in ids and b in ids and missing not in ids
    assert next(entry for entry in listing if entry['id'] == a)['visible'] is False
    reopened = call('open', '--id', a)
    assert reopened['visible'] and reopened['position'] == hidden['position'] and reopened['size'] == hidden['size']
    assert reopened['note']['color'] == '#112233'
    assert len(call('list')['windows']) == len(listing)
    # Old clients without id/color keep selecting the default note.
    legacy = raw({'action': 'inspect'})
    assert legacy['id'] == 'main' and legacy['note'] == default['note']
    stable(default, legacy)
    print('PASS: two independent windows, default compatibility, colors, invalid raw IPC atomicity, missing IDs, sorted list, repeated updates, focus/geometry/hidden preservation, reuse')
finally:
    for note_id in created:
        call('close', '--id', note_id)
