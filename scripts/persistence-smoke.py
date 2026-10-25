#!/usr/bin/env python3
"""Restart only the development process started here; preserve existing notes."""
import json
import pathlib
import subprocess
import time
import uuid

ROOT = pathlib.Path(__file__).resolve().parents[1]
CLI = ROOT / 'scripts/agent-gibbon-dev'

def cli(*args, check=True):
    result = subprocess.run([str(CLI), *args], cwd=ROOT, capture_output=True, text=True)
    value = json.loads(result.stdout)
    if check:
        assert result.returncode == 0 and value['ok'], value
    return value

assert not cli('list', check=False)['ok'], '개발 앱을 먼저 종료하세요. 기존 프로세스는 종료하지 않습니다.'
info = json.loads(subprocess.check_output(['node', 'scripts/dev.mjs', 'info'], cwd=ROOT))
process = None
files = []
ids = ['persist-' + uuid.uuid4().hex[:10], 'Persist-' + uuid.uuid4().hex[:10]]

def start():
    global process
    process = subprocess.Popen([str(CLI), '--serve'], cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(60):
        if cli('list', check=False)['ok']:
            return
        assert process.poll() is None, '개발 앱 시작 실패'
        time.sleep(.1)
    raise AssertionError('개발 앱 응답 시간 초과')

def stop():
    global process
    if process is not None:
        # dev.mjs forwards SIGTERM to its own child, whose binary was verified by the launcher.
        process.terminate()
        process.wait(timeout=10)
        process = None
        for _ in range(50):
            if not cli('list', check=False)['ok']:
                return
            time.sleep(.1)
        raise AssertionError('개발 앱 종료 실패')

try:
    start()
    for note_id in ids:
        value = cli('open', '--id', note_id, '--markdown', '# 영속 메모\n- [x] 한글\n', '--title', '복원 제목', '--color', '#abcdef')
        path = pathlib.Path(value['storage_path'])
        files.append(path)
        assert info['id'] in str(path)
        assert path.name == 'index.md'
        assert path.parent.parent.name == 'docs'
        assert path.read_text().endswith('# 영속 메모\n- [x] 한글\n')
        assert path.stat().st_mode & 0o777 == 0o600
        cli('close', '--id', note_id)
        cli('update', '--id', note_id, '--markdown', '숨긴 상태 갱신', '--status', '저장 완료')
    stop()
    start()
    for note_id in ids:
        value = cli('inspect', '--id', note_id)
        assert not value['visible']
        assert value['note']['markdown'] == '숨긴 상태 갱신'
        assert value['note']['title'] == '복원 제목'
        assert value['note']['color'] == '#abcdef'
        assert value['note']['status'] == '저장 완료'
    cli('open', '--id', ids[0])
    assert cli('inspect', '--id', ids[0])['note']['markdown'] == '숨긴 상태 갱신'
    print('PASS: Markdown files, permissions, isolated path, hidden update, restart restore, reopen')
finally:
    stop()
    for path in files:
        path.unlink(missing_ok=True)
        path.parent.rmdir()
