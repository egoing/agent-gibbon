import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { themeFor } from '../ui/theme.mjs';

async function setup(storage = new Map()) {
  const elements = new Map();
  const element = id => {
    if (!elements.has(id)) elements.set(id, {
      hidden: false, disabled: false, value: '', dataset: {}, listeners: {}, attributes: {},
      addEventListener(type, listener) { this.listeners[type] = listener; },
      setAttribute(key, value) { this.attributes[key] = value; }, focus() {},
    });
    return elements.get(id);
  };
  const styles = {};
  const events = {};
  const timers = [];
  const pointers = {};
  const calls = [];
  let note = { revision: 1, markdown: '# 원문', html: '<h1>원문</h1>', color: '#20252e' };
  let feedbackActive = false;
  let highlightRevision = null;
  let saveError;
  let warpError;
  let contact = false;
  const context = vm.createContext({
    localStorage: { getItem: key => storage.get(key), setItem: (key, value) => storage.set(key, value) },
    themeFor, clearTimeout() {}, setTimeout(callback) { timers.push(callback); },
    document: {
      getElementById: element, querySelectorAll: () => [],
      addEventListener(type, listener) { events[type] = listener; },
      documentElement: { style: { setProperty(key, value) { styles[key] = value; } }, addEventListener(type, listener) { pointers[type] = listener; } },
    },
    window: { __TAURI__: {
      core: { async invoke(command, args) {
        calls.push([command, args]);
        if (command === 'warp_note' && warpError) throw warpError;
        if (command === 'poll_warp') { const result = contact; contact = false; return result; }
        if (command === 'snapshot') return { note: { ...note }, always_on_top: true, feedback_active: feedbackActive, highlight_revision: highlightRevision };
        if (command === 'acknowledge_feedback') {
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
  return { element, styles, events, calls, pointers, highlight: value => { highlightRevision = value; }, refresh: () => timers.find(fn => fn.name === 'refresh')(),
    pollWarp: () => timers.find(fn => fn.name === 'pollWarp')(),
    feedback: active => { feedbackActive = active; },
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


test('feedback persists after opacity expiry until pointer contact, preserving drafts', async () => {
  const app = await setup();
  app.update({ revision: 2, markdown: '변경', html: '<p>변경</p>', change_html: '<p><mark>변경</mark></p>', removed: true });
  app.highlight(2); app.feedback(true); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p><mark>변경</mark></p>');
  assert.equal(app.element('change-notice').hidden, false);
  app.element('edit').listeners.click();
  app.element('markdown').value = '초안';
  app.feedback(false); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p><mark>변경</mark></p>');
  await app.pointers.pointerenter();
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  assert.equal(app.element('change-notice').hidden, true);
  app.highlight(2); await app.refresh(); // An already in-flight snapshot may be stale.
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
  assert.equal(app.element('markdown').value, '초안');
  app.update({ revision: 3, color: '#ffffff' }); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p>변경</p>');
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
});


test('next body replaces previous highlight and pointer movement acknowledges only that note', async () => {
  const app = await setup();
  const other = await setup();
  for (const instance of [app, other]) {
    instance.update({ revision: 2, markdown: '첫 변경', html: '<p>첫 변경</p>', change_html: '<p><mark>첫 변경</mark></p>' });
    instance.highlight(2); await instance.refresh();
  }
  app.update({ revision: 3, markdown: '다음 변경', html: '<p>다음 변경</p>', change_html: '<p><mark>다음 변경</mark></p>' });
  app.highlight(3); await app.refresh();
  assert.equal(app.element('content').innerHTML, '<p><mark>다음 변경</mark></p>');
  await app.pointers.pointermove();
  assert.equal(app.element('content').innerHTML, '<p>다음 변경</p>');
  assert.equal(other.element('content').innerHTML, '<p><mark>첫 변경</mark></p>');
});
