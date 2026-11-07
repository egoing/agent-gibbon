import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../ui/updater-notice.js', import.meta.url), 'utf8');
const html = readFileSync(new URL('../ui/updater-notice.html', import.meta.url), 'utf8');
const capability = JSON.parse(readFileSync(new URL('../src-tauri/capabilities/updater-events.json', import.meta.url), 'utf8'));
const tick = () => new Promise(setImmediate);

function setup(status) {
  const elements = new Map();
  const make = () => ({ hidden: false, disabled: false, textContent: '', attrs: {}, setAttribute(k, v) { this.attrs[k] = v; } });
  for (const id of ['notice-title', 'notice-version', 'notice-message', 'notice-install', 'notice-dismiss']) elements.set(id, make());
  const requests = [];
  const context = vm.createContext({
    document: { getElementById: id => elements.get(id) },
    window: { __TAURI__: { core: { invoke: async (command, args) => { requests.push({ command, args }); if (command === 'updater_notice_status') return status; if (command === 'updater_notice_ready') return {}; if (command === 'updater_notice_install') return status; if (command === 'updater_notice_dismiss') return { dismissed: true }; throw Error(`unexpected command ${command}`); } } } },
    setInterval: () => 1,
  });
  vm.runInContext(source, context);
  return { elements, requests };
}

test('ready notice exposes hold reason, safe install action and dismiss action', async () => {
  const app = setup({ phase: 'ready', version: '1.3.0', current_version: '1.2.3', message: '최근 입력이 있어 자동 설치를 보류했습니다.' });
  await tick();
  assert.equal(app.elements.get('notice-version').textContent, '새 버전 1.3.0');
  assert.equal(app.elements.get('notice-message').textContent, '최근 입력이 있어 자동 설치를 보류했습니다.');
  assert.equal(app.elements.get('notice-install').disabled, false);
  app.elements.get('notice-install').onclick();
  await tick();
  app.elements.get('notice-dismiss').onclick();
  await tick();
  assert.deepEqual(app.requests.map(request => request.command), ['updater_notice_status', 'updater_notice_ready', 'updater_notice_install', 'updater_notice_dismiss']);
});

test('notice page is guarded and exposes only notice-scoped update controls', () => {
  assert.match(html, /updater-guard\.js/);
  assert.match(html, /updater-notice\.js/);
  assert.doesNotMatch(source, /invoke\(['"](?:save_note|capture_screen|open_note|terminal_start)['"]/);
  assert.deepEqual(capability.windows, ['updater-notice']);
  assert.deepEqual(capability.permissions, ['core:event:allow-listen', 'core:event:allow-unlisten']);
});

test('compact ready notice hides routine explanation and omits verification label', async () => {
  const app = setup({ phase: 'ready', version: '1.3.0', message: '실행 중인 터미널이 있어 설치를 기다립니다.' });
  await tick();
  assert.match(app.elements.get('notice-version').textContent, /1\.3\.0/);
  assert.doesNotMatch(app.elements.get('notice-version').textContent, /서명 검증/);
  assert.equal(app.elements.get('notice-message').hidden, true);
  assert.doesNotMatch(html, /class="(?:eyebrow|detail)"/);
  assert.equal(app.elements.get('notice-message').textContent, '실행 중인 터미널이 있어 설치를 기다립니다.');
});

// Routine explanation is omitted; an unsuccessful explicit action still needs feedback.
test('explicit installation hold and status errors remain actionable', async () => {
  const app = setup({ phase: 'ready', version: '1.3.0', message: '편집을 저장한 뒤 다시 시도하세요.' });
  await tick();
  await app.elements.get('notice-install').onclick();
  assert.equal(app.elements.get('notice-message').hidden, false);
  assert.equal(app.elements.get('notice-message').textContent, '편집을 저장한 뒤 다시 시도하세요.');
  const error = setup({ phase: 'error', message: '설치를 시작하지 못했습니다.' });
  await tick();
  assert.equal(error.elements.get('notice-message').hidden, false);
  assert.equal(error.elements.get('notice-message').attrs.role, 'alert');
});
