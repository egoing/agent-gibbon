import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../ui/updater.js', import.meta.url), 'utf8');
const html = readFileSync(new URL('../ui/manager.html', import.meta.url), 'utf8');

function setup(initial = { phase: 'idle', version: null, current_version: '1.2.3', downloaded: 0, total: null, message: '' }) {
  const elements = new Map();
  const make = tagName => ({ tagName, hidden: false, disabled: false, textContent: '', attrs: {}, setAttribute(k, v) { this.attrs[k] = v; }, append() {} });
  for (const id of ['settings-page', 'settings-updater', 'updater-status', 'updater-version', 'updater-progress', 'updater-message', 'updater-check', 'updater-install']) elements.set(id, make());
  const requests = [];
  let status = initial;
  let check = async () => status;
  let install = async () => status;
  let interval;
  const context = vm.createContext({
    document: { hidden: false, getElementById: id => elements.get(id) },
    window: { __TAURI__: { core: { invoke: async (name, args) => { requests.push({ name, args }); if (name === 'updater_status') return status; if (name === 'updater_check') return check(); if (name === 'updater_install') return install(); throw Error(`unexpected command ${name}`); } } } },
    setInterval: fn => { interval = fn; },
  });
  vm.runInContext(source, context);
  return { elements, requests, run: code => vm.runInContext(code, context), tick: () => interval(), setStatus: value => { status = value; }, setCheck: fn => { check = fn; }, setInstall: fn => { install = fn; } };
}

const tick = () => new Promise(setImmediate);

test('manager exposes updater settings navigation and panel without a modal', () => {
  assert.match(html, /id="settings-updater-nav"/);
  assert.match(html, /id="settings-updater"/);
  assert.match(html, /id="updater-check"/);
  assert.match(html, /id="updater-install"/);
  assert.doesNotMatch(html, /updater.*role="dialog"/s);
});

test('status renders version and download progress, and installs only when ready', async () => {
  const app = setup({ phase: 'downloading', version: '1.3.0', current_version: '1.2.3', downloaded: 40, total: 100, message: '다운로드 중' });
  await app.run('window.gibbonUpdater.refresh()');
  assert.equal(app.requests[0].name, 'updater_status');
  assert.match(app.elements.get('updater-status').textContent, /다운로드/);
  assert.equal(app.elements.get('updater-version').textContent, '현재 버전 1.2.3 · 새 버전 1.3.0');
  assert.equal(app.elements.get('updater-progress').value, 40);
  assert.equal(app.elements.get('updater-install').disabled, true);
  app.setStatus({ phase: 'ready', version: '1.3.0', current_version: '1.2.3', downloaded: 100, total: 100, message: '안전하게 설치할 수 있습니다.' });
  await app.run('window.gibbonUpdater.refresh()');
  assert.equal(app.elements.get('updater-install').disabled, false);
});

test('debug disabled status explains that automatic updates are unavailable', async () => {
  const app = setup({ phase: 'disabled', version: null, current_version: '0.0.0-dev', downloaded: 0, total: null, message: 'debug 빌드에서는 자동 업데이트를 사용할 수 없습니다.' });
  await app.run('window.gibbonUpdater.refresh()');
  assert.match(app.elements.get('updater-message').textContent, /debug 빌드/);
  assert.equal(app.elements.get('updater-check').disabled, true);
  assert.equal(app.elements.get('updater-install').disabled, true);
});

test('check and install reject duplicate clicks and show returned safe-condition message', async () => {
  const app = setup({ phase: 'idle', version: null, current_version: '1.2.3', downloaded: 0, total: null, message: '' });
  await app.run('window.gibbonUpdater.refresh()');
  let resolveCheck;
  app.setCheck(() => new Promise(resolve => { resolveCheck = resolve; }));
  const check = app.elements.get('updater-check').onclick();
  assert.equal(app.elements.get('updater-check').disabled, true);
  await app.elements.get('updater-check').onclick();
  assert.equal(app.requests.filter(request => request.name === 'updater_check').length, 1);
  resolveCheck({ phase: 'ready', version: '1.3.0', current_version: '1.2.3', downloaded: 100, total: 100, message: '터미널 작업이 끝난 뒤 다시 시도하세요.' });
  await check;
  assert.equal(app.elements.get('updater-install').disabled, false);
  let resolveInstall;
  app.setInstall(() => new Promise(resolve => { resolveInstall = resolve; }));
  const install = app.elements.get('updater-install').onclick();
  await app.elements.get('updater-install').onclick();
  assert.equal(app.requests.filter(request => request.name === 'updater_install').length, 1);
  resolveInstall({ phase: 'ready', version: '1.3.0', current_version: '1.2.3', downloaded: 100, total: 100, message: '편집을 저장한 뒤 다시 시도하세요.' });
  await install;
  assert.equal(app.elements.get('updater-message').textContent, '편집을 저장한 뒤 다시 시도하세요.');
});

test('periodic refresh runs only while updater settings are visible', async () => {
  const app = setup();
  app.elements.get('settings-page').hidden = true;
  await app.tick(); await tick();
  assert.equal(app.requests.length, 0);
  app.elements.get('settings-page').hidden = false;
  app.elements.get('settings-updater').hidden = false;
  await app.tick(); await tick();
  assert.equal(app.requests.filter(request => request.name === 'updater_status').length, 1);
  app.elements.get('settings-updater').hidden = true;
  await app.tick(); await tick();
  assert.equal(app.requests.filter(request => request.name === 'updater_status').length, 1);
});

test('request errors use safe text and expose an alert', async () => {
  const app = setup(); app.setCheck(async () => { throw Error('<img src=x>'); });
  await app.elements.get('updater-check').onclick();
  assert.equal(app.elements.get('updater-message').textContent, '업데이트를 확인하지 못했습니다. 다시 시도하세요.');
  assert.equal(app.elements.get('updater-message').attrs.role, 'alert');
});

test('compact settings hide unavailable installation and empty detail', async () => {
  const app = setup();
  await app.run('window.gibbonUpdater.refresh()');
  assert.equal(app.elements.get('updater-install').hidden, true);
  assert.equal(app.elements.get('updater-message').hidden, true);
  app.setStatus({ phase: 'ready', current_version: '1.2.3', version: '1.3.0', message: '편집을 저장한 뒤 다시 시도하세요.' });
  await app.run('window.gibbonUpdater.refresh()');
  assert.equal(app.elements.get('updater-install').hidden, false);
  assert.equal(app.elements.get('updater-message').hidden, false);
  assert.equal(app.elements.get('updater-install').disabled, false);
  app.setStatus({ phase: 'preparing', version: '1.3.0', message: '작업 상태 확인 중' });
  await app.run('window.gibbonUpdater.refresh()');
  assert.equal(app.elements.get('updater-install').hidden, false);
  assert.equal(app.elements.get('updater-install').disabled, true);
  app.setStatus({ phase: 'error', message: '서명 검증 실패' });
  await app.run('window.gibbonUpdater.refresh()');
  assert.equal(app.elements.get('updater-install').hidden, true);
  assert.equal(app.elements.get('updater-message').attrs.role, 'alert');
});
