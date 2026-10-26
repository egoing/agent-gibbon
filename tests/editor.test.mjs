import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { themeFor } from '../ui/theme.mjs';

async function setup(storage = new Map(), instance = {}, initialNote = {}) {
  const elements = new Map();
  const element = id => {
    if (!elements.has(id)) elements.set(id, {
      hidden: false, disabled: false, value: '', dataset: {}, listeners: {}, attributes: {},
      addEventListener(type, listener) { this.listeners[type] = listener; },
      setAttribute(key, value) { this.attributes[key] = value; }, focus() {},
      getBoundingClientRect() { return { width: 298, height: this.hidden ? 0 : 42 }; },
      cloneNode() { return { style: {}, getBoundingClientRect: () => ({ width: 298, height: 120 }), remove() {} }; },
    });
    return elements.get(id);
  };
  const styles = {};
  const events = {};
  const nativeEvents = {};
  const timers = [];
  const pointers = {};
  const calls = [];
  let note = { revision: 1, markdown: '# 원문', html: '<h1>원문</h1>', color: '#20252e', ...initialNote };
  let feedbackActive = false;
  let highlightRevision = null;
  let saveError;
  let acknowledgementError;
  let warpError;
  let contact = false;
  let folded = false;
  const classes = new Set();
  const context = vm.createContext({
    localStorage: { getItem: key => storage.get(key), setItem: (key, value) => storage.set(key, value) },
    themeFor, clearTimeout() {}, setTimeout(callback) { timers.push(callback); },
    document: {
      body: { appendChild() {}, classList: { toggle(name, value) { if (value) classes.add(name); else classes.delete(name); } } },
      getElementById: element, querySelectorAll: () => [],
      addEventListener(type, listener) { events[type] = listener; },
      documentElement: { style: { setProperty(key, value) { styles[key] = value; } }, addEventListener(type, listener) { pointers[type] = listener; } },
    },
    window: { __TAURI__: {
      event: { async listen(name, handler) { nativeEvents[name] = handler; return () => {}; } },
      core: { async invoke(command, args) {
        calls.push([command, args]);
        if (command === 'poll_fold') return folded;
        if (command === 'toggle_fold') { folded = !folded; return folded; }
        if (command === 'warp_note' && warpError) throw warpError;
        if (command === 'poll_warp') { const result = contact; contact = false; return result; }
        if (command === 'snapshot') return { ...instance, note: { ...note }, always_on_top: true, feedback_active: feedbackActive, highlight_revision: highlightRevision };
        if (command === 'acknowledge_feedback') {
          if (acknowledgementError) throw acknowledgementError;
          if (highlightRevision !== args.revision) return false;
          highlightRevision = null; return true;
        }
        if (command === 'save_note') {
          if (saveError) throw saveError;
          assert.equal(args.expectedRevision, note.revision);
          note = { ...note, markdown: args.markdown, html: args.markdown === note.markdown ? note.html : '<p>저장됨</p>', revision: note.revision + 1 };
          return note;
        }
      } },
      window: { getCurrentWindow: () => ({ startDragging: async () => calls.push(['drag']) }) },
    } },
  });
  vm.runInContext(readFileSync(new URL('../ui/app.js', import.meta.url), 'utf8').replace("import { themeFor } from './theme.mjs';", ''), context);
  await new Promise(setImmediate);
  return { element, styles, events, calls, pointers, classes, nativeEvents,
    fold: value => { folded = value; },
    pollFold: () => timers.find(fn => fn.name === 'pollFold')(), highlight: value => { highlightRevision = value; }, refresh: () => timers.find(fn => fn.name === 'refresh')(),
    pollWarp: () => timers.find(fn => fn.name === 'pollWarp')(),
    feedback: active => { feedbackActive = active; },
    failAcknowledgement: error => { acknowledgementError = error; },
    contact: () => { contact = true; }, failWarp: error => { warpError = error; },
    update: patch => { note = { ...note, ...patch }; }, failSave: error => { saveError = error; } };
}

test('font sizes cycle four steps and remain selected across note updates', async () => {
  const app = await setup();
  for (const size of [14, 16, 18, 12]) {
    app.element('font-size').listeners.click();
    assert.equal(app.styles['--font-size'], `${size}px`);
    assert.match(app.element('font-size').attributes['aria-label'], new RegExp(`${size}px`));
  }
  app.element('font-size').listeners.click();
  app.update({ revision: 2, markdown: '갱신' });
  await app.refresh();
  assert.equal(app.styles['--font-size'], '14px');
});

test('editing preserves drafts on polling and failed saves; cancel reveals latest note', async () => {
  const app = await setup();
  app.element('edit').listeners.click();
  assert.equal(app.element('markdown').value, '# 원문');
  app.element('markdown').value = '작성 중';
  app.update({ revision: 2, html: '<p>외부 갱신</p>' });
  await app.refresh();
  assert.equal(app.element('markdown').value, '작성 중');
  app.failSave('메모 충돌');
  await app.element('save').listeners.click();
  await app.refresh();
  assert.equal(app.element('error').hidden, false);
  assert.equal(app.element('error').textContent, '메모 충돌');
  assert.equal(app.element('markdown').value, '작성 중');
  assert.equal(app.element('editor').hidden, false);
  app.element('cancel').listeners.click();
  assert.equal(app.element('editor').hidden, true);
  assert.equal(app.element('content').innerHTML, '<p>외부 갱신</p>');
});

test('save returns to rendered content; editor input never drags the window; Escape cancels', async () => {
  const app = await setup();
  app.element('edit').listeners.click();
  app.events.mousedown({ button: 0, target: { closest: selector => selector.includes('#editor') }, preventDefault() { assert.fail('editor must accept native selection'); } });
  assert.equal(app.calls.some(([name]) => name === 'drag'), false);
  app.element('markdown').value = '편집됨';
  await app.element('save').listeners.click();
  assert.equal(app.element('content').innerHTML, '<p>저장됨</p>');
  assert.equal(app.element('editor').hidden, true);
  app.element('edit').listeners.click();
  assert.equal(app.element('markdown').value, '편집됨');
  app.events.keydown({ key: 'Escape', preventDefault() {} });
  assert.equal(app.element('editor').hidden, true);
  assert.equal(app.calls.some(([name]) => name === 'hide_note'), false);
});


test('warp click/contact show help; dismissal persists across notes and reloads', async () => {
  const storage = new Map();
  const app = await setup(storage);
  app.element('warp-tip').hidden = true;
  await app.element('warp').listeners.click();
  assert.equal(app.calls.some(([name]) => name === 'warp_note'), true);
  assert.equal(app.element('warp-tip').hidden, false);
  app.element('warp-tip-close').listeners.click();
  assert.equal(app.element('warp-tip').hidden, true);
  app.contact(); await app.pollWarp();
  assert.equal(app.element('warp-tip').hidden, false);
  app.element('warp-tip-dismiss').listeners.click();
  assert.equal(app.element('warp-tip').hidden, true);
  const another = await setup(storage);
  another.element('warp-tip').hidden = true;
  await another.element('warp').listeners.click();
  assert.equal(another.element('warp-tip').hidden, true);
  app.contact(); await app.pollWarp();
  assert.equal(app.element('warp-tip').hidden, true);
});

test('failed warp reports error without showing success help or losing draft', async () => {
  const app = await setup();
  app.element('warp-tip').hidden = true;
  app.element('edit').listeners.click();
  app.element('markdown').value = '작성 중';
  app.failWarp('화면이 작습니다');
  await app.element('warp').listeners.click();
  assert.equal(app.element('error').textContent, '화면이 작습니다');
  assert.equal(app.element('warp-tip').hidden, true);
  assert.equal(app.element('markdown').value, '작성 중');
});


test('feedback persists after opacity expiry and pointer contact until button click, preserving drafts', async () => {
  const app = await setup();
  app.update({ revision: 2, markdown: '변경', html: '<p>변경</p>', change_html: '<p><mark>변경</mark></p>', removed: true });
  app.highlight(2); app.feedback(true); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p><mark>변경</mark></p>');
  app.element('edit').listeners.click();
  app.element('markdown').value = '초안';
  app.feedback(false); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p><mark>변경</mark></p>');
  await app.pointers.pointerenter();
  assert.equal(app.element('content').innerHTML, '<p><mark>변경</mark></p>');
  await app.element('clear-diff').listeners.click();
  app.highlight(2); await app.refresh(); // Ignore a stale snapshot after acknowledgement.
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  assert.equal(app.element('markdown').value, '초안');
  app.update({ revision: 3, color: '#ffffff' }); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  assert.equal(app.element('error').hidden, true);
});


test('feedback never reappears when an expired change is saved without editing', async () => {
  const app = await setup();
  app.update({ revision: 2, markdown: '변경', html: '<p>변경</p>', change_html: '<p><mark>변경</mark></p>' });
  app.feedback(false); await app.refresh();
  app.element('edit').listeners.click();
  await app.element('save').listeners.click();
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  assert.equal(app.element('error').hidden, true);
});


test('next body persists through rollover until explicit button click and affects only that note', async () => {
  const app = await setup();
  const other = await setup();
  for (const instance of [app, other]) {
    instance.update({ revision: 2, markdown: '첫 변경', html: '<p>첫 변경</p>', change_html: '<p><mark>첫 변경</mark></p>' });
    instance.highlight(2); await instance.refresh();
  }
  await app.pointers.pointerenter();
  app.update({ revision: 3, markdown: '다음 변경', html: '<p>다음 변경</p>', change_html: '<p><mark>다음 변경</mark></p>' });
  app.highlight(3); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p><mark>다음 변경</mark></p>');
  await app.pointers.pointermove?.();
  await app.events.focus?.();
  await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p><mark>다음 변경</mark></p>');
  await app.pointers.pointerleave();
  await app.pointers.pointerenter();
  assert.equal(app.element('content').innerHTML, '<p><mark>다음 변경</mark></p>');
  assert.equal(app.element('clear-diff').hidden, false);
  await app.element('clear-diff').listeners.click();
  assert.equal(app.element('clear-diff').hidden, true);
  assert.equal(app.element('content').innerHTML, '<p>다음 변경</p>');
  assert.equal(other.element('content').innerHTML, '<p><mark>첫 변경</mark></p>');
});


test('fold contact preserves editor drafts and restores through the unfold button', async () => {
  const app = await setup();
  app.element('edit').listeners.click();
  app.element('markdown').value = '저장하지 않은 초안';
  app.update({ title: '작은 제목' });
  await app.refresh();
  app.fold(true);
  await app.pollFold();
  assert.equal(app.classes.has('folded'), true);
  assert.equal(app.element('fold-toggle').hidden, false);
  assert.equal(app.element('fold-title').textContent, '작은 제목');
  assert.equal(app.element('fold-title').hidden, false);
  app.update({ revision: 2, markdown: 'CLI 갱신' });
  await app.refresh();
  await app.pollFold();
  assert.equal(app.classes.has('folded'), true);
  assert.equal(app.element('markdown').value, '저장하지 않은 초안');
  await app.element('fold-toggle').listeners.click();
  assert.equal(app.classes.has('folded'), false);
  assert.equal(app.element('fold-toggle').attributes['aria-label'], '메모 접기');
  assert.equal(app.element('fold-title').hidden, true);
  assert.equal(app.element('editor').hidden, false);
  assert.equal(app.element('markdown').value, '저장하지 않은 초안');
});


test('explicit button folds and unfolds without Control and updates its label', async () => {
  const app = await setup();
  await app.element('fold-toggle').listeners.click();
  assert.equal(app.classes.has('folded'), true);
  assert.equal(app.element('fold-toggle').attributes['aria-label'], '메모 펼치기');
  assert.equal(app.element('fold-toggle').attributes['aria-expanded'], 'false');
  assert.match(app.element('fold-toggle').title, /Control.*포인터.*접기·펼치기/);
  await app.element('fold-toggle').listeners.click();
  assert.equal(app.classes.has('folded'), false);
  assert.equal(app.element('fold-toggle').attributes['aria-label'], '메모 접기');
  assert.equal(app.element('fold-toggle').attributes['aria-expanded'], 'true');
});


test('development labels and tip preferences stay separate from installed notes', async () => {
  const storage = new Map();
  const installed = await setup(storage);
  const a = await setup(storage, { instance: 'dev-a', dev_label: 'DEV same · aaaaaaaa' });
  const b = await setup(storage, { instance: 'dev-b', dev_label: 'DEV same · bbbbbbbb' });
  assert.equal(installed.element('dev-instance').hidden, true);
  assert.equal(a.element('dev-instance').hidden, false);
  assert.equal(a.element('dev-instance').textContent, 'DEV same · aaaaaaaa');
  a.update({ title: '제목' });
  await a.refresh();
  assert.equal(a.element('fold-title').textContent, '[DEV same · aaaaaaaa] 제목');
  a.element('warp-tip-dismiss').listeners.click();
  assert.equal(storage.get('floatnote.warp-tip-dismissed:dev-a'), 'true');
  assert.equal(storage.has('floatnote.warp-tip-dismissed'), false);
  assert.equal(storage.has('floatnote.warp-tip-dismissed:dev-b'), false);
  assert.equal(b.element('dev-instance').textContent, 'DEV same · bbbbbbbb');
});

test('native rollover updates chrome without a DOM click or pointer event', async () => {
  const app = await setup();
  const html = app.element('content').innerHTML;
  app.nativeEvents['note-pointer-over']({ payload: true });
  assert.equal(app.classes.has('pointer-over'), true);
  app.nativeEvents['note-pointer-over']({ payload: false });
  assert.equal(app.classes.has('pointer-over'), false);
  assert.equal(app.element('content').innerHTML, html);
  assert.equal(app.calls.some(([name]) => name === 'drag_note'), false);
});

test('clear diff button stays hidden without changes and stale clicks preserve newer revisions', async () => {
  const app = await setup();
  assert.equal(app.element('clear-diff').hidden, true);
  app.update({ revision: 2, markdown: '변경', html: '<p>변경</p>', change_html: '<p><mark>변경</mark></p>' });
  app.highlight(2); await app.refresh();
  app.highlight(3); // A newer backend update arrived before the button request.
  await app.element('clear-diff').listeners.click();
  assert.equal(app.element('clear-diff').hidden, false);
  assert.equal(app.element('content').innerHTML, '<p><mark>변경</mark></p>');
  assert.equal(app.element('clear-diff').disabled, false);
});


test('bottom action reserves height and releases it when changes are cleared', async () => {
  const app = await setup();
  assert.equal(app.element('diff-actions').hidden, true);
  app.update({ revision: 2, markdown: '변경', html: '<p>변경</p>', change_html: '<p><mark>변경</mark></p>' });
  app.highlight(2); await app.refresh();
  assert.equal(app.element('diff-actions').hidden, false);
  assert.equal(app.calls.filter(([name]) => name === 'fit_note').at(-1)[1].height, 164);
  app.events.mousedown({ button: 0, target: { closest: selector => selector.includes('#diff-actions') }, preventDefault() { assert.fail('menu must not drag the note'); } });
  assert.equal(app.calls.some(([name]) => name === 'drag_note'), false);
  await app.element('clear-diff').listeners.click();
  assert.equal(app.element('diff-actions').hidden, true);
  assert.equal(app.calls.filter(([name]) => name === 'fit_note').at(-1)[1].height, 122);
});


test('failed confirmation keeps cumulative changes and allows retry', async () => {
  const app = await setup();
  app.update({ revision: 2, markdown: '변경', html: '<p>변경</p>', change_html: '<p><mark>누적 변경</mark></p>' });
  app.highlight(2); await app.refresh();
  app.failAcknowledgement('메모 저장 실패');
  await app.element('clear-diff').listeners.click();
  assert.equal(app.element('content').innerHTML, '<p><mark>누적 변경</mark></p>');
  assert.equal(app.element('diff-actions').hidden, false);
  assert.equal(app.element('clear-diff').disabled, false);
  assert.equal(app.element('error').textContent, '메모 저장 실패');
  app.failAcknowledgement(null);
  await app.element('clear-diff').listeners.click();
  assert.equal(app.element('diff-actions').hidden, true);
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  assert.equal(app.element('error').hidden, true);
});

test('folded note opens from title, blank space, border and icon without starting a drag', async () => {
  const app = await setup();
  for (const id of ['fold-title', 'viewport', 'body', 'fold-toggle']) {
    app.fold(true);
    await app.pollFold();
    let stopped = false;
    const event = { button: 0, target: app.element(id),
      preventDefault() {}, stopPropagation() { stopped = true; } };
    const before = app.calls.filter(([name]) => name === 'toggle_fold').length;
    app.events.mousedown(event);
    await app.events.click(event);
    if (!stopped && id === 'fold-toggle') await app.element(id).listeners.click();
    assert.equal(stopped, true);
    assert.equal(app.classes.has('folded'), false);
    assert.equal(app.calls.filter(([name]) => name === 'toggle_fold').length, before + 1);
  }
  assert.equal(app.calls.filter(([name]) => name === 'drag_note').length, 0);
});

test('folded click ignores secondary buttons and coalesces pending clicks', async () => {
  const app = await setup();
  app.fold(true);
  await app.pollFold();
  for (const button of [1, 2]) {
    app.events.click({ button, preventDefault() { assert.fail('non-primary click'); } });
  }
  assert.equal(app.classes.has('folded'), true);
  const event = { button: 0, preventDefault() {}, stopPropagation() {} };
  await Promise.all([app.events.click(event), app.events.click(event)]);
  assert.equal(app.calls.filter(([name]) => name === 'toggle_fold').length, 1);
  assert.equal(app.classes.has('folded'), false);
  await app.events.click({ button: 0, preventDefault() { assert.fail('expanded content click'); } });
  assert.equal(app.calls.filter(([name]) => name === 'toggle_fold').length, 1);
});


test('region text note opens a blank editor once, preserves drafts and saves normally', async () => {
  const app = await setup(new Map(), {}, { edit_on_open: true, markdown: '', html: '' });
  assert.equal(app.element('editor').hidden, false);
  assert.equal(app.element('markdown').value, '');
  assert.equal(app.calls.some(([name]) => name === 'fit_note'), false);
  app.element('markdown').value = '새 메모';
  await app.refresh();
  assert.equal(app.element('markdown').value, '새 메모');
  await app.element('save').listeners.click();
  assert.equal(app.element('editor').hidden, true);
  await app.refresh();
  assert.equal(app.element('editor').hidden, true);
  assert.equal(app.calls.find(([name]) => name === 'save_note')[1].markdown, '새 메모');
});

test('canceling the initial region editor does not reopen it on polling', async () => {
  const app = await setup(new Map(), {}, { edit_on_open: true, markdown: '', html: '' });
  app.element('cancel').listeners.click();
  await app.refresh();
  assert.equal(app.element('editor').hidden, true);
});


test('direct save immediately clears existing diff controls', async () => {
  const app = await setup();
  app.highlight(1);
  await app.refresh();
  assert.equal(app.element('diff-actions').hidden, false);
  app.element('edit').listeners.click();
  app.element('markdown').value = '직접 수정';
  await app.element('save').listeners.click();
  assert.equal(app.element('diff-actions').hidden, true);
  assert.equal(app.element('clear-diff').hidden, true);
  app.update({ revision: 3, html: '<p>외부 수정</p>', change_html: '<mark>외부 수정</mark>' });
  app.highlight(3);
  await app.refresh();
  assert.equal(app.element('diff-actions').hidden, false);
});
