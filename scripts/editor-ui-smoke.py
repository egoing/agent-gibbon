#!/usr/bin/env python3
"""Exercise editor controls on macOS; touches only unique notes and temporary files.
Set AGENT_GIBBON_TEST_BIN / AGENT_GIBBON_TEST_PROCESS for an isolated app instance.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
BIN = os.environ.get('AGENT_GIBBON_TEST_BIN', str(ROOT / 'scripts/agent-gibbon'))
PROCESS = os.environ.get('AGENT_GIBBON_TEST_PROCESS', 'agent-gibbon')

def apple(code):
    return subprocess.check_output(['osascript', '-e', code], text=True, timeout=15).strip()

def ui(code):
    return apple(f'tell application "System Events" to tell process {json.dumps(PROCESS)} to {code}')

def cli(action, note_id, *args):
    data = json.loads(subprocess.check_output([BIN, action, '--id', note_id, *args], text=True, timeout=25))
    assert data['ok'], data
    return data

def root(note_id):
    return 'group "Agent Gibbon" of UI element 1 of scroll area 1 of group 1 of group 1 of window ' + json.dumps('Agent Gibbon — ' + note_id, ensure_ascii=False)

def editor(note_id):
    return 'group "Markdown 편집" of ' + root(note_id)

def click(control, target):
    ui(f'click {control} of {target}')
    time.sleep(.45)

def write(note_id, text):
    ui('set focused of text area "Markdown 원문" of ' + editor(note_id) + ' to true')
    ui('set value of text area "Markdown 원문" of ' + editor(note_id) + ' to ' + json.dumps(text, ensure_ascii=False))
    assert draft(note_id) == text, 'Accessibility text input was not applied'

def draft(note_id):
    return ui('get value of text area "Markdown 원문" of ' + editor(note_id))

def contents(note_id):
    return ui('get entire contents of window ' + json.dumps('Agent Gibbon — ' + note_id, ensure_ascii=False))

def passed(message):
    print('PASS: ' + message, flush=True)

front = apple('tell application "System Events" to get name of first application process whose frontmost is true')
ids = ['editor-test-' + uuid.uuid4().hex[:8] for _ in range(2)]
a, b = ids
created = []
try:
    with tempfile.TemporaryDirectory(prefix='agent-gibbon-editor-smoke-') as folder:
        path = Path(folder) / '한글 메모.md'
        path.write_text('# 원본\n- [ ] 편집 대기')
        first = cli('open', a, '--file', str(path)); created.append(a)
        assert 'markdown' in first['note'], 'Running app is an old build: editor unavailable'
        second = cli('open', b, '--markdown', '다른 메모'); created.append(b)
        time.sleep(.8)
        for size in (14, 16, 18, 12):
            click('button 1', root(a))
            assert f'글자 크기 {size}px' in ui('get description of button 1 of ' + root(a))
        dimensions = [ui('get size of button ' + str(i) + ' of ' + root(a)) for i in (1, 2, 3)]
        assert len(set(dimensions)) == 1, dimensions
        assert '12px' in ui('get description of button 1 of ' + root(b))
        click('button 1', root(a))
        cli('update', a, '--status', '갱신'); cli('close', a); cli('open', a); time.sleep(.4)
        assert '14px' in ui('get description of button 1 of ' + root(a))
        passed('four font sizes, equal button dimensions, per-window size and hide/update persistence')
        click('button "메모 편집"', root(a))
        text = '# 편집 완료\n- [x] 한글 파일 저장'
        write(a, text); click('button "저장"', editor(a))
        assert path.read_text() == text
        assert cli('inspect', a)['note']['markdown'] == text
        assert '☑ 한글 파일 저장' in contents(a)
        assert cli('inspect', b)['note']['markdown'] == '다른 메모'
        passed('pencil edit, Korean source file save, rendered checklist, window isolation')
        click('button "메모 편집"', root(a)); write(a, '취소할 내용')
        click('button "취소"', editor(a)); assert path.read_text() == text
        click('button "메모 편집"', root(a)); write(a, '숨긴 초안')
        cli('close', a); cli('open', a); time.sleep(.4)
        assert draft(a) == '숨긴 초안'
        click('button "취소"', editor(a))
        passed('cancel preserves file; hidden draft survives reopen')
        click('button "메모 편집"', root(a)); write(a, '충돌 초안')
        cli('update', a, '--status', '외부 갱신'); time.sleep(.5)
        assert draft(a) == '충돌 초안'
        click('button "저장"', editor(a)); time.sleep(.5)
        assert '메모가 외부에서 갱신되었습니다' in contents(a)
        assert draft(a) == '충돌 초안' and path.read_text() == text
        click('button "취소"', editor(a))
        click('button "메모 편집"', root(a)); write(a, '파일 충돌 초안')
        path.write_text('외부 파일 변경'); click('button "저장"', editor(a))
        assert '원본 파일이 외부에서 변경되었습니다' in contents(a)
        assert draft(a) == '파일 충돌 초안' and path.read_text() == '외부 파일 변경'
        click('button "취소"', editor(a))
        passed('CLI and file conflicts preserve drafts and external changes')
        cli('update', a, '--file', str(path)); time.sleep(.5)
        click('button "메모 편집"', root(a)); write(a, '[ ] 잘못된 체크리스트')
        click('button "저장"', editor(a)); assert 'MARKDOWN_TASK_MARKER' in contents(a)
        assert path.read_text() == '외부 파일 변경'
        write(a, '- [x] 교정'); click('button "저장"', editor(a))
        assert path.read_text() == '- [x] 교정', {'file': path.read_text(), 'ui': contents(a), 'note': cli('inspect', a)}
        passed('validation error keeps file; correction can be saved')
        cli('update', a, '--markdown', '파일 연결 해제'); time.sleep(.5)
        click('button "메모 편집"', root(a)); write(a, '')
        click('button "저장"', editor(a))
        assert cli('inspect', a)['note']['markdown'] == ''
        assert path.read_text() == '- [x] 교정'
        for key in ('position', 'size', 'always_on_top'):
            assert cli('inspect', a)[key] == first[key], key
            assert cli('inspect', b)[key] == second[key], key
        passed('inline edit/empty save leaves detached file intact; window geometry/pin preserved')
finally:
    for note_id in created:
        cli('close', note_id)
    apple('tell application "System Events" to set frontmost of process ' + json.dumps(front, ensure_ascii=False) + ' to true')
