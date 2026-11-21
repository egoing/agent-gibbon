import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

// Guard the startup wiring as well as the separately tested idle policy: a cached
// download must never turn a background attempt into an explicit user request.
test('cached startup and every background retry use automatic installation gates', () => {
  const source = readFileSync(new URL('../src-tauri/src/updater.rs', import.meta.url), 'utf8');
  const startup = source.slice(source.indexOf('pub fn initialize('), source.indexOf('#[cfg(target_os = "macos")]\nfn idle_seconds'));
  assert.match(startup, /restore_cache\(&app\)/);
  assert.match(startup, /install\(&app, true\)/);
  assert.doesNotMatch(startup, /install\(&app, (?!true\))[^)]+\)/);
});
