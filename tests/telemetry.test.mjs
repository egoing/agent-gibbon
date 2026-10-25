import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';

test('frontend reports only fixed error kinds and handles IPC failure', async () => {
  const listeners = {};
  const calls = [];
  const window = {
    addEventListener: (kind, listener) => { listeners[kind] = listener; },
    __TAURI__: { core: { invoke: (command, args) => {
      calls.push({ command, args });
      return Promise.reject(new Error('offline'));
    } } },
  };
  vm.runInNewContext(readFileSync(new URL('../ui/telemetry.js', import.meta.url), 'utf8'), { window });
  for (const kind of ['error', 'unhandledrejection']) {
    listeners[kind]({ message: 'private note', reason: 'private path' });
  }
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(JSON.stringify(calls), JSON.stringify([
    { command: 'report_frontend_error', args: { kind: 'error' } },
    { command: 'report_frontend_error', args: { kind: 'unhandledrejection' } },
  ]));
});
