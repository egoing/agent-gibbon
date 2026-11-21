import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

test('native activation distinguishes passive creation, deliberate content focus and explicit selection', { skip: process.platform !== 'darwin' }, () => {
  const root = fileURLToPath(new URL('..', import.meta.url));
  const dir = mkdtempSync(join(tmpdir(), 'jajan-passive-creation-'));
  try {
    const binary = join(dir, 'test');
    const build = spawnSync('clang', ['-fobjc-arc', '-fblocks', '-Wno-deprecated-declarations',
      '-framework', 'Cocoa', '-framework', 'Carbon', 'tests/passive-creation.m',
      'src-tauri/src/app_activation.m', '-o', binary], { cwd: root, encoding: 'utf8' });
    assert.equal(build.status, 0, build.stderr);
    const run = spawnSync(binary, [], { encoding: 'utf8', timeout: 10000 });
    assert.equal(run.status, 0, run.stderr);
    assert.match(run.stdout, /^PASS:/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
