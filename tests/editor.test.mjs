import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { memoThemeFor } from '../ui/theme.mjs';
import { readMemoAppearance } from '../ui/appearance.mjs';
import { feedbackView, durations } from '../ui/feedback-state.mjs';

async function setup(storage = new Map(), instance = {}, initialNote = {}, platform = "MacIntel") {
  const elements = new Map();
  const element = id => {
    if (!elements.has(id)) elements.set(id, {
      hidden: false, disabled: false, value: '', dataset: {}, listeners: {}, attributes: {},
      closest(selector) { return id === 'fold-toggle' && selector.includes('button') ? this : null; },
      addEventListener(type, listener) { this.listeners[type] = listener; },
      setAttribute(key, value) { this.attributes[key] = value; }, removeAttribute(key) { delete this.attributes[key]; }, focus() {},
      setSelectionRange(start, end) { this.selectionStart = start; this.selectionEnd = end; },
      getBoundingClientRect() { return { width: 298, height: this.hidden ? 0 : (id === 'controls' ? 30 : 42) }; },
      cloneNode() { return { style: {}, querySelector: () => ({ hidden: false, innerHTML: '', remove() {} }), getBoundingClientRect: () => ({ width: 298, height: 120 }), remove() {} }; },
    });
    return elements.get(id);
  };
  const styles = {};
  const events = {};
  const nativeEvents = {};
  const nativeTargets = {};
  const timers = [];
  const pointers = {};
  const calls = [];
  let note = { revision: 1, markdown: '# 원문', html: '<h1>원문</h1>', color: '#20252e', ...initialNote };
  let feedbackActive = false;
  let highlightRevision = null;
  let saveError;
  let saveDelay;
  let exportResult = null;
  let exportError;
  let acknowledgementError;
  let warpError;
  let copyError;
  let contact = false;
  let folded = false;
  const classes = new Set();
  const context = vm.createContext({
    navigator: { platform },
    localStorage: { getItem: key => storage.get(key), setItem: (key, value) => storage.set(key, value) },
    memoThemeFor, readMemoAppearance, feedbackView, durations, clearTimeout() {}, setTimeout(callback) { timers.push(callback); },
    document: {
      createElement() { return { style: {}, setAttribute() {}, remove() {}, get innerText() { return this.innerHTML.replace(/<[^>]+>/g, '').trim(); } }; },
      createElement() { return { style: {}, setAttribute() {}, remove() {},
        get innerText() { return this.innerHTML.replace(/<[^>]+>/g, '').trim(); } }; },
      body: { appendChild() {}, classList: { toggle(name, value) { if (value) classes.add(name); else classes.delete(name); } } },
      getElementById: element, querySelectorAll: () => [],
      addEventListener(type, listener) { events[type] = listener; },
      documentElement: { style: { setProperty(key, value) { styles[key] = value; } }, addEventListener(type, listener) { pointers[type] = listener; } },
    },
    window: { __TAURI__: {
      clipboardManager: { async writeText(text) { if (copyError) throw copyError; calls.push(['clipboard-text', text]); } },
      event: { async listen(name, handler, options) { nativeEvents[name] = handler; nativeTargets[name] = options?.target; return () => {}; } },
      core: { async invoke(command, args) {
        calls.push([command, args]);
        if (command === 'poll_fold') return folded;
        if (command === 'toggle_fold') { folded = !folded; return folded; }
        if (command === 'warp_note' && warpError) throw warpError;
        if (command === 'poll_warp') { const result = contact; contact = false; return result; }
        if (command === 'snapshot') return { ...instance, note: { ...note }, always_on_top: true, feedback_active: feedbackActive, highlight_revision: highlightRevision };
        if (command === 'acknowledge_feedback') {
          if (acknowledgementError) throw acknowledgementError;
          if (note.revision !== args.revision) return false;
          highlightRevision = null; note.completion_html = null; return true;
        }
        if (command === 'export_note') { if (exportError) throw exportError; return exportResult; }
        if (command === 'save_note') {
          if (saveDelay) await saveDelay;
          if (saveError) throw saveError;
          if (args.expectedRevision !== note.revision) throw new Error('메모 충돌');
          note = { ...note, markdown: args.markdown, html: args.markdown === note.markdown ? note.html : '<p>저장됨</p>', revision: note.revision + 1 };
          return note;
        }
      } },
      window: { getCurrentWindow: () => ({ label: 'memo-test', startDragging: async () => calls.push(['drag']) }) },
    } },
  });
  context.window.localStorage = context.localStorage;
  vm.runInContext(readFileSync(new URL('../ui/app.js', import.meta.url), 'utf8').replace(/^import .*;$/gm, ''), context);
  await new Promise(setImmediate);
  const action = async payload => { await nativeEvents['feedback-action']({ payload: { owner: 'memo-test', action: payload, revision: view().revision } }); await new Promise(setImmediate); };
  const view = () => calls.filter(([name]) => name === 'present_feedback').at(-1)?.[1].view;
  return { toggleFromToolbar: () => context.window.gibbonToggleNote(), exportResult: value => { exportResult = value; }, failExport: error => { exportError = error; }, failCopy: error => { copyError = error; }, element, styles, events, calls, pointers, classes, nativeEvents, nativeTargets, action, view,
    expireNotice: async () => { timers.filter(fn => fn.name === 'clearNotice').at(-1)?.(); await new Promise(setImmediate); },
    fold: value => { folded = value; },
    pollFold: () => timers.find(fn => fn.name === 'pollFold')(), highlight: value => { highlightRevision = value; }, refresh: () => timers.find(fn => fn.name === 'refresh')(),
    pollWarp: () => timers.find(fn => fn.name === 'pollWarp')(),
    feedback: active => { feedbackActive = active; },
    leave: async () => { instance.focused = true; await timers.find(fn => fn.name === 'refresh')(); instance.focused = false; await timers.find(fn => fn.name === 'refresh')(); },
    state: patch => Object.assign(instance, patch),
    failAcknowledgement: error => { acknowledgementError = error; },
    contact: () => { contact = true; }, failWarp: error => { warpError = error; },
    update: patch => { note = { ...note, ...patch }; }, delaySave: promise => { saveDelay = promise; }, failSave: error => { saveError = error; } };
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
  await app.action('save');
  await app.refresh();
  assert.equal(!app.view().error, false);
  assert.equal(app.view().message, '메모 충돌');
  assert.equal(app.element('markdown').value, '작성 중');
  assert.equal(app.element('editor').hidden, false);
  await app.action('cancel');
  assert.equal(app.element('editor').hidden, true);
  assert.equal(app.element('content').innerHTML, '<p>외부 갱신</p>');
});

test('save returns to rendered content; editor input never drags the window; Escape cancels', async () => {
  const app = await setup();
  app.element('edit').listeners.click();
  app.events.mousedown({ button: 0, target: { closest: selector => selector.includes('#editor') }, preventDefault() { assert.fail('editor must accept native selection'); } });
  assert.equal(app.calls.some(([name]) => name === 'drag'), false);
  app.element('markdown').value = '편집됨';
  await app.action('save');
  assert.equal(app.element('content').innerHTML, '<p>저장됨</p>');
  assert.equal(app.element('editor').hidden, true);
  app.element('edit').listeners.click();
  assert.equal(app.element('markdown').value, '편집됨');
  app.events.keydown({ key: 'Escape', preventDefault() {} });
  assert.equal(app.element('editor').hidden, true);
  assert.equal(app.calls.some(([name]) => name === 'hide_note'), false);
});


test('failed warp reports error without showing success help or losing draft', async () => {
  const app = await setup();
  app.element('edit').listeners.click();
  app.element('markdown').value = '작성 중';
  app.failWarp('화면이 작습니다');
  await app.element('warp').listeners.click();
  assert.equal(app.view().message, '화면이 작습니다');
  assert.equal(app.element('markdown').value, '작성 중');
});


test('automatic acknowledgement defers during editing and preserves drafts', async () => {
  const app = await setup();
  app.update({ revision: 2, markdown: '변경', html: '<p>변경</p>', change_html: '<p><mark>변경</mark></p>', removed: true });
  app.highlight(2); app.feedback(true); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p><mark>변경</mark></p>');
  app.element('edit').listeners.click();
  app.element('markdown').value = '초안';
  app.feedback(false); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p><mark>변경</mark></p>');
  await app.leave();
  assert.equal(app.element('content').innerHTML, '<p><mark>변경</mark></p>');
  await app.leave(); // Editing must keep the unread revision.
  assert.equal(app.element('content').innerHTML, '<p><mark>변경</mark></p>');
  await app.action('cancel');
  await app.leave();
  app.highlight(2); await app.refresh(); // Ignore a stale snapshot after acknowledgement.
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  assert.equal(app.element('markdown').value, '초안');
  app.update({ revision: 3, color: '#ffffff' }); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  assert.equal(!app.view().error, true);
});


test('feedback never reappears when an expired change is saved without editing', async () => {
  const app = await setup();
  app.update({ revision: 2, markdown: '변경', html: '<p>변경</p>', change_html: '<p><mark>변경</mark></p>' });
  app.feedback(false); await app.refresh();
  app.element('edit').listeners.click();
  await app.action('save');
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  assert.equal(!app.view().error, true);
});


test('rollover and focus acknowledge only their own current revision without a button', async () => {
  const app = await setup();
  const other = await setup();
  for (const instance of [app, other]) {
    instance.update({ revision: 2, html: '<p>변경</p>', change_html: '<mark>변경</mark>' });
    instance.highlight(2); await instance.refresh();
    assert.deepEqual(instance.view().buttons, []);
  }
  await app.leave();
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  assert.equal(other.element('content').innerHTML, '<mark>변경</mark>');
  app.update({ revision: 3, html: '<p>다음</p>', change_html: '<mark>다음</mark>' });
  app.highlight(3); await app.refresh();
  await app.leave();
  assert.equal(app.element('content').innerHTML, '<p>다음</p>');
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
  assert.equal(app.element('fold-toggle').title, '메모 펼치기');
  assert.equal(app.view().message, '');
  assert.equal(app.view().folded_tip, undefined);
  await app.element('fold-toggle').listeners.click();
  assert.equal(app.classes.has('folded'), false);
  assert.equal(app.element('fold-toggle').attributes['aria-label'], '메모 접기');
  assert.equal(app.element('fold-toggle').attributes['aria-expanded'], 'true');
});


test('native rollover updates chrome without a DOM click or pointer event', async () => {
  const app = await setup();
  assert.equal(app.nativeTargets['note-pointer-over']?.kind, 'Window');
  assert.equal(app.nativeTargets['note-pointer-over']?.label, 'memo-test');
  const html = app.element('content').innerHTML;
  app.nativeEvents['note-pointer-over']({ payload: true });
  assert.equal(app.classes.has('pointer-over'), true);
  app.nativeEvents['note-pointer-over']({ payload: false });
  assert.equal(app.classes.has('pointer-over'), false);
  assert.equal(app.element('content').innerHTML, html);
  assert.equal(app.calls.some(([name]) => name === 'drag_note'), false);
});

test('native pointer notifications are addressed to the originating memo', () => {
  // The JS handler cannot distinguish a broadcast boolean from its own hover.
  // Guard the Rust/JS wiring as well as the handler behavior tested above.
  const backend = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
  const syncPointer = backend.slice(backend.indexOf('fn sync_pointer('), backend.indexOf('// Native alpha'));
  assert.match(syncPointer, /\.emit_to\(window\.label\(\), "note-pointer-over", hovered\)/);
  assert.doesNotMatch(syncPointer, /\.emit\("note-pointer-over"/);
});

test('clear diff button stays hidden without changes and stale clicks preserve newer revisions', async () => {
  const app = await setup();
  assert.equal((app.view().revision === null), true);
  app.update({ revision: 2, markdown: '변경', html: '<p>변경</p>', change_html: '<p><mark>변경</mark></p>' });
  app.highlight(2); await app.refresh();
  app.update({ revision: 3 });
  app.highlight(3); // A newer backend update arrived before reading.
  app.state({ focused: true }); await app.refresh();
  assert.equal((app.view().revision === null), false);
  assert.equal(app.element('content').innerHTML, '<p><mark>변경</mark></p>');
  assert.deepEqual(app.view().buttons, []);
});


test('external action never consumes memo height', async () => {
  const app = await setup();
  assert.equal((app.view().revision === null), true);
  app.update({ revision: 2, markdown: '변경', html: '<p>변경</p>', change_html: '<p><mark>변경</mark></p>' });
  app.highlight(2); await app.refresh();
  assert.equal((app.view().revision === null), false);
  assert.equal(app.calls.filter(([name]) => name === 'fit_note').at(-1)[1].height, 152);
  assert.equal(app.calls.some(([name]) => name === 'drag_note'), false);
  await app.leave();
  assert.equal((app.view().revision === null), true);
  assert.equal(app.calls.filter(([name]) => name === 'fit_note').at(-1)[1].height, 152);
});


test('failed confirmation keeps cumulative changes and allows retry', async () => {
  const app = await setup();
  app.update({ revision: 2, markdown: '변경', html: '<p>변경</p>', change_html: '<p><mark>누적 변경</mark></p>' });
  app.highlight(2); await app.refresh();
  app.failAcknowledgement('메모 저장 실패');
  await app.leave();
  assert.equal(app.element('content').innerHTML, '<p><mark>누적 변경</mark></p>');
  assert.equal((app.view().revision === null), false);
  assert.deepEqual(app.view().buttons, []);
  assert.equal(app.view().message, '메모 저장 실패');
  app.failAcknowledgement(null);
  await app.leave();
  await app.expireNotice();
  assert.equal((app.view().revision === null), true);
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  assert.equal(!app.view().error, true);
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
  await app.action('save');
  assert.equal(app.element('editor').hidden, true);
  await app.refresh();
  assert.equal(app.element('editor').hidden, true);
  assert.equal(app.calls.find(([name]) => name === 'save_note')[1].markdown, '새 메모');
});

test('canceling the initial region editor does not reopen it on polling', async () => {
  const app = await setup(new Map(), {}, { edit_on_open: true, markdown: '', html: '' });
  await app.action('cancel');
  await app.refresh();
  assert.equal(app.element('editor').hidden, true);
});


test('direct save immediately clears existing diff controls', async () => {
  const app = await setup();
  app.highlight(1);
  await app.refresh();
  assert.equal((app.view().revision === null), false);
  app.element('edit').listeners.click();
  app.element('markdown').value = '직접 수정';
  await app.action('save');
  assert.equal((app.view().revision === null), true);
  assert.equal((app.view().revision === null), true);
  app.update({ revision: 3, html: '<p>외부 수정</p>', change_html: '<mark>외부 수정</mark>' });
  app.highlight(3);
  await app.refresh();
  assert.equal((app.view().revision === null), false);
});

test('temporary copy notice restores acknowledgement; editing owns the action slot', async () => {
  const app = await setup();
  app.highlight(1); await app.refresh();
  await app.element('copy').listeners.click();
  assert.equal(app.view().message, '복사했습니다');
  assert.deepEqual(app.view().buttons.map(b => b.id), []);
  await app.expireNotice();
  assert.equal(app.view().message, '');
  app.element('edit').listeners.click();
  await new Promise(setImmediate);
  await app.element('warp').listeners.click();
  assert.deepEqual(app.view().buttons.map(b => b.id), ['cancel', 'save']);
  assert.match(app.view().message, /Option/);
  await app.action('cancel');
  assert.deepEqual(app.view().buttons.map(b => b.id), []);
});

test('save success becomes Toast; save error retains actions and outranks tips', async () => {
  const app = await setup();
  app.element('edit').listeners.click();
  app.failSave('원본 파일이 외부에서 변경되었습니다');
  await app.action('save');
  await app.element('warp').listeners.click();
  assert.equal(app.view().error, true);
  assert.match(app.view().message, /원본 파일/);
  assert.deepEqual(app.view().buttons.map(b => b.id), ['cancel', 'save']);
  app.failSave(null);
  await app.action('save');
  assert.equal(app.view().message, '저장했습니다');
  assert.equal(app.view().buttons.length, 0);
  await app.expireNotice();
  assert.equal(app.view().message, '');
});

test('move icon follows the destination direction returned by the current monitor', async () => {
  const app = await setup(new Map(), { warp_right: true });
  assert.equal(app.element('warp').attributes['data-direction'], 'right');
  app.state({ warp_right: false });
  await app.refresh();
  assert.equal(app.element('warp').attributes['data-direction'], 'left');
});

test('development metadata does not alter the displayed memo title', async () => {
  const app = await setup(new Map(), { dev_label: 'DEV design-system', instance: 'dev-a' }, { title: '검증 메모' });
  assert.equal(app.element('fold-title').textContent, '검증 메모');
  assert.equal(app.element('content').innerHTML, '<h1>원문</h1>');
});

test('delayed surface acknowledgement cannot acknowledge a newer rendered revision', async () => {
  const app = await setup();
  app.highlight(2); await app.refresh();
  app.highlight(3); await app.refresh();
  await app.nativeEvents['feedback-action']({ payload: { owner: 'memo-test', action: 'acknowledge', revision: 2 } });
  assert.equal(app.calls.some(([name]) => name === 'acknowledge_feedback'), false);
});

test('hidden memo defers companion creation until its native window is visible', async () => {
  const instance = { visible: false };
  const app = await setup(new Map(), instance);
  app.highlight(2); await app.refresh();
  assert.equal(app.calls.some(([name]) => name === 'present_feedback'), false);
  instance.visible = true;
  await app.refresh();
  assert.deepEqual(app.view().buttons.map(b => b.id), []);
});

 test('foreign Feedback actions cannot save another memo draft', async () => {
  const app = await setup();
  app.element('edit').listeners.click();
  app.element('markdown').value = '보존할 초안';
  await app.nativeEvents['feedback-action']({ payload: { owner: 'other-window', action: 'save' } });
  assert.equal(app.element('editor').hidden, false);
  assert.equal(app.element('markdown').value, '보존할 초안');
});

test('frame opens only its own connected edge and restores after hiding', async () => {
  const app = await setup();
  const connection = app.nativeEvents['feedback-connection'];
  connection({ payload: { owner: 'other', edge: 'below' } });
  assert.equal(app.classes.has('feedback-below'), false);
  connection({ payload: { owner: 'memo-test', edge: 'below', width: 100, memoWidth: 300, memoHeight: 160 } });
  assert.equal(app.classes.has('feedback-below'), true);
  assert.equal(app.classes.has('feedback-above'), false);
  connection({ payload: { owner: 'memo-test', edge: 'above', width: 300, memoWidth: 300, memoHeight: 160 } });
  assert.equal(app.classes.has('feedback-above'), true);
  assert.equal(app.classes.has('feedback-below'), false);
  connection({ payload: { owner: 'memo-test', edge: 'none' } });
  assert.equal(app.classes.has('feedback-below'), false);
  assert.equal(app.classes.has('feedback-above'), false);
});


test('copy uses the latest rendered note without confirming or copying deleted diff text', async () => {
  const app = await setup(undefined, {}, { html: '<p>최신 본문</p>', change_html: '<del>이전 본문</del><p>최신 본문</p>' });
  app.highlight(1);
  await app.refresh();
  const before = app.element('content').innerHTML;
  await app.element('copy').listeners.click();
  assert.deepEqual(app.calls.find(([name]) => name === 'clipboard-text'), ['clipboard-text', '최신 본문']);
  assert.equal(app.element('content').innerHTML, before);
  assert.deepEqual(app.view().buttons.map(b => b.id), []);
  assert.equal(app.calls.some(([name]) => name === 'acknowledge_feedback'), false);
  assert.equal(app.view().message, '복사했습니다');
  app.events.keydown({ key: 'c', metaKey: true, preventDefault() {} });
  await new Promise(setImmediate);
  assert.equal(app.calls.filter(([name]) => name === 'clipboard-text').length, 2);
  await app.expireNotice();
  assert.equal(app.view().message, '');
  app.element('edit').listeners.click();
  let prevented = false;
  app.events.keydown({ key: 'c', ctrlKey: true, preventDefault() { prevented = true; } });
  assert.equal(prevented, false);
  assert.equal(app.calls.filter(([name]) => name === 'clipboard-text').length, 2);
});

test('capture copy invokes native image writing without fetching a data URL', async () => {
  const app = await setup();
  app.update({ capture: { width: 100, height: 50 } });
  await app.refresh();
  await app.element('copy').listeners.click();
  assert.equal(app.calls.filter(([name]) => name === 'copy_note_image').length, 1);
  assert.equal(app.calls.some(([name]) => name === 'clipboard-text'), false);
  assert.equal(app.view().message, '복사했습니다');
});

test('failed clipboard write reports the error without showing success feedback', async () => {
  const app = await setup();
  app.failCopy('클립보드 실패');
  await app.element('copy').listeners.click();
  assert.equal(app.view().message, '클립보드 실패');
  assert.equal(app.view().error, true);
});


test('Cmd+S and Ctrl+S export viewing notes but editing blocks toolbar export', async () => {
  const app = await setup();
  for (const modifier of ['metaKey', 'ctrlKey']) {
    let prevented = false;
    app.events.keydown({ key: 's', [modifier]: true, preventDefault() { prevented = true; } });
    await new Promise(setImmediate);
    assert.equal(prevented, true);
    assert.equal(app.calls.at(-1)[0], 'export_note');
    assert.equal(app.calls.at(-1)[1].markdown, null);
  }
  app.element('edit').listeners.click();
  app.element('markdown').value = '# 저장할 초안';
  app.exportResult('/tmp/note.md');
  await app.element('export').listeners.click();
  await new Promise(setImmediate);
  assert.equal(app.calls.filter(([name]) => name === 'export_note').length, 2);
  assert.equal(app.element('export').hidden, true);
  assert.equal(app.element('editor').hidden, false);
  assert.equal(app.view().buttons.map(b => b.label).join(','), '취소,저장');
  assert.equal(app.calls.some(([name]) => name === 'save_note'), false);
});

test('reading export failure preserves note and allows retry', async () => {
  const app = await setup();
  app.failExport('쓰기 실패');
  await app.element('export').listeners.click();
  await new Promise(setImmediate);
  assert.equal(app.view().message, '쓰기 실패');
  assert.equal(app.element('export').disabled, false);
});

test('repeated shortcuts open only one export dialog', async () => {
  const app = await setup();
  const event = { key: 's', ctrlKey: true, preventDefault() {} };
  app.events.keydown(event);
  app.events.keydown(event);
  await new Promise(setImmediate);
  assert.equal(app.calls.filter(([name]) => name === 'export_note').length, 1);
});

test('stale Feedback export action cannot export an editing draft', async () => {
  const app = await setup();
  app.element('edit').listeners.click();
  app.element('markdown').value = '내보낼 초안';
  await app.nativeEvents['feedback-action']({ payload: { owner: 'other', action: 'export' } });
  assert.equal(app.calls.some(([name]) => name === 'export_note'), false);
  await app.nativeEvents['feedback-action']({ payload: { owner: 'memo-test', action: 'export' } });
  assert.equal(app.calls.some(([name]) => name === 'export_note'), false);
  await app.action('cancel');
  assert.equal(app.element('export').hidden, false);
  await app.element('export').listeners.click();
  assert.equal(app.calls.filter(([name]) => name === 'export_note').at(-1)[1].markdown, null);
  app.element('edit').listeners.click();
  assert.equal(app.element('editor').hidden, false);
});


test('toolbar shortcuts follow platform and gesture help is shown on button use', async () => {
  for (const [platform, modifier] of [['MacIntel', 'Command '], ['Win32', 'Ctrl ']]) {
    const app = await setup(new Map(), {}, {}, platform);
    assert.equal(app.element('export').title, `파일 내보내기 · ${modifier}S`);
    assert.equal(app.element('copy').title, `본문 복사 · ${modifier}C`);
    await app.element('warp').listeners.click();
    if (platform === 'MacIntel') assert.match(app.view().message, /Option.*포인터/);
    else assert.equal(app.view().message, '');
  }
});


test('polling and hover never reset placement intent, explicit editing does', async () => {
  const app = await setup();
  await app.refresh(); await app.pollFold(); await app.pollWarp();
  await app.leave(); await app.pointers.pointerleave();
  assert.equal(app.calls.filter(([name]) => name === 'content_action').length, 0);
  app.element('edit').listeners.click();
  assert.equal(app.calls.filter(([name]) => name === 'content_action').length, 1);
  app.element('markdown').listeners.input();
  assert.equal(app.calls.filter(([name]) => name === 'content_action').length, 2);
});

test('content measurement carries the current revision and pending reopen can measure in editor', async () => {
  const app = await setup(new Map(), { layout_pending: true }, { edit_on_open: true, revision: 9 });
  const fit = app.calls.find(([name]) => name === 'fit_note');
  assert.equal(fit[1].revision, 9);
  assert.equal(app.element('editor').hidden, false);
  assert.equal(app.calls.filter(([name]) => name === 'content_action').length, 0);
});


test('window close terminates while Escape remains a hide action', async () => {
  const app = await setup();
  app.element('close').listeners.click();
  assert.equal(app.calls.at(-1)[0], 'close_note');
  app.events.keydown({key:'Escape',preventDefault(){},target:{closest(){return null;}}});
  assert.equal(app.calls.at(-1)[0], 'hide_note');
});

test('rollover and focus entry preserve completion; departure clears it', async () => {
  const app = await setup(new Map(), {}, { html: '<p>완료</p>', completion_html: '<mark>완료</mark>' });
  await app.pointers.pointerenter();
  app.state({ focused: true }); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<mark>완료</mark>');
  app.state({ focused: false }); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p>완료</p>');
});
test('unseen revision arriving on departure survives until another reading session', async () => {
  const app = await setup(new Map(), { focused: true });
  app.state({ focused: false });
  app.update({ revision: 2, html: '<p>new</p>', change_html: '<mark>new</mark>' });
  app.highlight(2); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<mark>new</mark>');
  await app.leave();
  assert.equal(app.element('content').innerHTML, '<p>new</p>');
});
test('companion focus continues the same reading session', async () => {
  const app = await setup(new Map(), { focused: true });
  app.update({ revision: 2, html: 'new', change_html: '<mark>new</mark>' });
  app.highlight(2); await app.refresh();
  app.state({ focused: false, feedback_surface: { focused: true } }); await app.refresh();
  assert.equal(app.view().revision, 2);
  app.state({ feedback_surface: { focused: false } }); await app.refresh();
  assert.equal(app.view().revision, null);
});

test('independent memo designs preserve editor drafts and original colors without a note revision',async()=>{
 const storage=new Map();const app=await setup(storage,{}, {color:'#ffeeaa'});
 assert.equal(app.styles['--background'],'#dfe2dd');
 app.element('edit').listeners.click();app.element('markdown').value='보존할 초안';
 storage.set('agent-gibbon.manager-theme','black');await app.refresh();assert.equal(app.styles['--background'],'#dfe2dd');
 for(const [design,bg] of [['white','#f8f8f8'],['black','#262627'],['color','#ffeeaa'],['system','#dfe2dd'],['gray','#dfe2dd']]){
  storage.set('agent-gibbon.memo-design',design);await app.refresh();assert.equal(app.styles['--background'],bg);assert.equal(app.element('markdown').value,'보존할 초안');
 }
 assert(!app.calls.some(([name])=>name==='save_note'));
});


test('shared toolbar reopens note feedback through the normal note toggle', async () => {
  const app = await setup();
  await app.toggleFromToolbar();
  await new Promise(setImmediate);
  assert.equal(app.classes.has('folded'), true);
  assert.equal(app.view().message, '');
  await app.toggleFromToolbar();
  await new Promise(setImmediate);
  assert.equal(app.classes.has('folded'), false);
  assert.match(app.view().message, /Control.*포인터/);
  assert.equal(app.view().folded_tip, undefined);
});

test('collapsed toolbar title starts native dragging without unfolding or stealing button clicks', async () => {
  const app = await setup();
  app.fold(true);
  await app.pollFold();
  let prevented = false;
  app.events.mousedown({button: 0, target: {closest: selector => selector === '#controls'},
    preventDefault() { prevented = true; }});
  await new Promise(setImmediate);
  assert.equal(prevented, true);
  assert.equal(app.calls.filter(([name]) => name === 'drag_note').length, 1);
  assert.equal(app.classes.has('folded'), true);
  app.events.mousedown({button: 0, target: {closest: selector => selector.includes('button')},
    preventDefault() { assert.fail('button interaction must be preserved'); }});
  assert.equal(app.calls.filter(([name]) => name === 'drag_note').length, 1);
});


test('editing save shortcuts share the save button path on both platforms and coalesce repeats', async () => {
  for (const platform of ['MacIntel', 'Win32']) {
    const app = await setup(new Map(), {}, {}, platform);
    app.element('edit').listeners.click();
    app.element('markdown').value = '반영할 초안';
    const event = {key: 's', [platform === 'MacIntel' ? 'metaKey' : 'ctrlKey']: true, preventDefault() {}};
    app.events.keydown(event);
    app.events.keydown({...event, repeat: true});
    await app.action('save-shortcut');
    assert.equal(app.calls.filter(([name]) => name === 'save_note').length, 1);
    assert.equal(app.calls.some(([name]) => name === 'export_note'), false);
    assert.equal(app.element('editor').hidden, true);
    assert.equal(app.view().message, '저장했습니다');
  }
});

test('IME and modified shortcuts never commit drafts; failed save retains draft for retry', async () => {
  const app = await setup();
  app.element('edit').listeners.click();
  app.element('markdown').value = '보존할 초안';
  for (const extra of [{isComposing: true}, {keyCode: 229}, {repeat: true}, {altKey: true}, {shiftKey: true}]) {
    app.events.keydown({key: 's', metaKey: true, preventDefault() {}, ...extra});
  }
  assert.equal(app.calls.some(([name]) => name === 'save_note'), false);
  app.failSave('원본 충돌');
  await app.action('save-shortcut');
  assert.equal(app.element('markdown').value, '보존할 초안');
  assert.equal(app.element('editor').hidden, false);
  assert.equal(app.element('markdown').readOnly, false);
  assert.equal(app.view().message, '원본 충돌');
  app.failSave(null);
  await app.action('save-shortcut');
  assert.equal(app.element('editor').hidden, true);
});

test('cancelled editor and image notes keep shortcut export behavior', async () => {
  for (const capture of [null, {path: 'image.png'}]) {
    const app = await setup(new Map(), {}, {capture});
    if (!capture) { app.element('edit').listeners.click(); await app.action('cancel'); }
    await app.action('save-shortcut');
    assert.equal(app.calls.filter(([name]) => name === 'export_note').at(-1)[1].markdown, null);
    assert.equal(app.calls.some(([name]) => name === 'save_note'), false);
  }
});

test('revision changed during editing keeps the old edit baseline and draft', async () => {
  const app = await setup(new Map(), {}, {source_path: '/tmp/test-note.md'});
  app.element('edit').listeners.click();
  app.element('markdown').value = '사용자 초안';
  app.update({revision: 2, markdown: 'CLI가 변경한 본문'});
  await app.refresh();
  await app.action('save-shortcut');
  assert.equal(app.calls.filter(([name]) => name === 'save_note').at(-1)[1].expectedRevision, 1);
  assert.equal(app.element('markdown').value, '사용자 초안');
  assert.equal(app.element('editor').hidden, false);
  assert.match(app.view().message, /메모 충돌/);
  assert.equal(app.calls.some(([name]) => name === 'export_note'), false);
});

test('pending save rejects duplicate save/export/cancel and locks only the editor', async () => {
  const app = await setup();
  app.element('edit').listeners.click();
  app.element('markdown').value = '느린 저장';
  let finish;
  app.delaySave(new Promise(resolve => { finish = resolve; }));
  const pending = app.action('save-shortcut');
  await new Promise(setImmediate);
  assert.equal(app.element('markdown').readOnly, true);
  await app.action('save-shortcut'); await app.action('save');
  await app.action('export'); await app.action('cancel');
  assert.equal(app.calls.filter(([name]) => name === 'save_note').length, 1);
  assert.equal(app.calls.some(([name]) => name === 'export_note'), false);
  assert.equal(app.element('editor').hidden, false);
  finish(); await pending; await new Promise(setImmediate);
  assert.equal(app.element('markdown').readOnly, false);
  assert.equal(app.element('editor').hidden, true);
});

test('editing keeps the toolbar blank area draggable and excludes editor/buttons', async () => {
  const app = await setup();
  app.element('edit').listeners.click();
  const before = app.calls.filter(([name]) => name === 'drag_note').length;
  app.events.mousedown({button: 0, target: {closest: () => null}, preventDefault() {}});
  assert.equal(app.calls.filter(([name]) => name === 'drag_note').length, before + 1);
  app.events.mousedown({button: 0, target: {closest: () => ({})}, preventDefault() {}});
  assert.equal(app.calls.filter(([name]) => name === 'drag_note').length, before + 1);
});

test('linked editing shows only actions by default and retains actual notices', () => {
  const state = {editing: true, saving: false, exporting: false, sourcePath: '/tmp/note.md'};
  assert.equal(feedbackView(state).message, '');
  assert.deepEqual(feedbackView(state).buttons.map(b => b.label), ['취소', '저장']);
  const error = feedbackView({...state, notice: {text: '원본 충돌', kind: 'error'}});
  assert.equal(error.message, '원본 충돌');
  assert.equal(error.error, true);
  assert.equal(feedbackView({...state, editing: false, notice: {text: '저장했습니다', kind: 'result'}}).message, '저장했습니다');
});
