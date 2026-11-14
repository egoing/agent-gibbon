#!/usr/bin/env python3
"""This checkout's CLI only; temporary source, own note/PID, no native UI actions."""
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import uuid

ROOT = Path(__file__).resolve().parents[1]
INFO = json.loads(subprocess.check_output(['node', str(ROOT / 'scripts/dev.mjs'), 'info'], text=True))
assert INFO['root'] == str(ROOT)
BIN = ROOT / 'scripts/agent-gibbon-dev'
ID = 'milkdown174-cli-' + uuid.uuid4().hex[:8]

def pids():
    rows = subprocess.check_output(['ps', '-axo', 'pid=,comm='], text=True).splitlines()
    return {int(parts[0]) for row in rows if len(parts := row.strip().split(None, 1)) == 2 and parts[1] == INFO['binary']}

before = pids()
records = {'root': str(ROOT), 'instance': INFO['id'], 'binary': INFO['binary'], 'note_id': ID, 'native_gui': 'not tested', 'commands': []}
created = False

def cli(action, *args):
    value = json.loads(subprocess.check_output([str(BIN), action, '--id', ID, *args], text=True))
    assert value['ok'], value
    assert value['instance'] == INFO['id'] and value['id'] == ID, value
    records['commands'].append({'action': action, 'ok': value['ok'], 'instance': value['instance'], 'visible': value['visible'], 'revision': value['note']['revision']})
    return value

try:
    with tempfile.TemporaryDirectory(prefix='gibbon174-' + INFO['id'] + '-') as folder:
        source = Path(folder) / 'source.md'
        original = '# ' + ID + '\n\n- [ ] 전용 CLI 검증\n'
        source.write_text(original)
        first = cli('open', '--file', str(source)); created = True
        inspected = cli('inspect')
        assert inspected['note']['markdown'] == original
        assert inspected['note']['source_path'] == str(source.resolve())
        assert inspected['focused'] is False
        closed = cli('close')
        state = cli('inspect')
        assert state['visible'] is False
        source.write_text('# ' + ID + '\n\n- [x] 숨긴 상태 갱신\n')
        cli('update', '--file', str(source))
        updated = cli('inspect')
        assert updated['visible'] is False
        assert updated['position'] == state['position'] and updated['size'] == state['size']
        assert updated['note']['markdown'] == source.read_text()
        assert updated['note']['revision'] > inspected['note']['revision']
        records['source_path'] = str(source.resolve())
        records['hidden_geometry_preserved'] = True
        records['initial_background_focus_preserved'] = True
        records['created_pids'] = sorted(pids() - before)
finally:
    if created:
        cli('close')
    for pid in pids() - before:
        # Recheck the executable immediately before termination.
        command = subprocess.check_output(['ps', '-p', str(pid), '-o', 'comm='], text=True).strip()
        if command == INFO['binary']:
            os.kill(pid, signal.SIGTERM)
print(json.dumps(records, ensure_ascii=False, indent=2))
