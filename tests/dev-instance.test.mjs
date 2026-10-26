import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, symlinkSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { devInstance, devEnvironment } from '../scripts/dev-instance.mjs';

test('canonical paths, not cwd or basename, determine stable instance IDs', () => {
  const temp = mkdtempSync(join(tmpdir(), 'floatnote-test-'));
  try {
    for (const dir of ['a/same', 'b/same']) mkdirSync(join(temp, dir), { recursive: true });
    symlinkSync(join(temp, 'a/same'), join(temp, 'alias'), process.platform === 'win32' ? 'junction' : 'dir');
    const a = devInstance(join(temp, 'a/same'));
    assert.equal(a.id, 'dev-' + createHash('sha256').update(a.root).digest('hex').slice(0, 24));
    assert.deepEqual(devInstance(join(temp, 'alias')), a);
    assert.notEqual(devInstance(join(temp, 'b/same')).id, a.id);
    assert.equal(a.binary, join(a.root, `src-tauri/target/debug/agent-gibbon${process.platform === 'win32' ? '.exe' : ''}`));
    const env = devEnvironment(a, { FLOATNOTE_INSTANCE: 'wrong', CARGO_TARGET_DIR: '/shared', KEEP: 'yes' });
    assert.equal(env.FLOATNOTE_INSTANCE, a.id);
    assert.equal(env.CARGO_TARGET_DIR, a.target);
    assert.equal(env.KEEP, 'yes');
  } finally { rmSync(temp, { recursive: true, force: true }); }
});

test('development CLI rejects missing, old and foreign binaries without release fallback', { skip: process.platform === 'win32' && 'POSIX executable fixtures; Windows uses native CLI smoke tests' }, async () => {
  const { copyFileSync, writeFileSync } = await import('node:fs');
  const { spawnSync } = await import('node:child_process');
  const temp = mkdtempSync(join(tmpdir(), 'floatnote launcher '));
  try {
    mkdirSync(join(temp, 'scripts'));
    for (const file of ['dev.mjs', 'dev-instance.mjs']) {
      copyFileSync(new URL(`../scripts/${file}`, import.meta.url), join(temp, 'scripts', file));
    }
    for (const profile of ['debug', 'release']) mkdirSync(join(temp, 'src-tauri/target', profile), { recursive: true });
    const run = () => spawnSync(process.execPath, [join(temp, 'scripts/dev.mjs'), 'cli', 'open', '--markdown', '한글 $literal'], {
      cwd: '/', encoding: 'utf8', env: { ...process.env, FLOATNOTE_INSTANCE: 'foreign', CARGO_TARGET_DIR: '/foreign' },
    });
    writeFileSync(join(temp, 'src-tauri/target/release/agent-gibbon'), '#!/bin/sh\necho release-must-not-run\n', { mode: 0o755 });
    assert.equal(run().status, 1);
    const binary = join(temp, 'src-tauri/target/debug/agent-gibbon');
    writeFileSync(binary, '#!/bin/sh\necho \'{"ok":false,"error":"old binary"}\'\n', { mode: 0o755 });
    assert.match(JSON.parse(run().stdout).error, /현재 워크트리/);
    writeFileSync(binary, `#!/usr/bin/env node
console.log(JSON.stringify({ok:true,instance:process.env.FLOATNOTE_INSTANCE,manifest_dir:'/foreign'}));
`, { mode: 0o755 });
    assert.equal(run().status, 1);
    const instance = devInstance(temp);
    writeFileSync(binary, `#!/usr/bin/env node
const info={ok:true,instance:process.env.FLOATNOTE_INSTANCE,manifest_dir:${JSON.stringify(join(instance.root, 'src-tauri'))}};
console.log(JSON.stringify(process.argv[2] === '--instance-info' ? info : {...info,args:process.argv.slice(2),cwd:process.cwd(),target:process.env.CARGO_TARGET_DIR}));
`, { mode: 0o755 });
    const accepted = run();
    assert.equal(accepted.status, 0, accepted.stderr);
    const response = JSON.parse(accepted.stdout);
    assert.equal(response.instance, instance.id);
    assert.equal(response.target, instance.target);
    assert.equal(response.cwd, '/');
    assert.deepEqual(response.args, ['open', '--markdown', '한글 $literal']);
  } finally { rmSync(temp, { recursive: true, force: true }); }
});
