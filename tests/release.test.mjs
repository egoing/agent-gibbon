import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, copyFileSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { checkVersion, setVersion } from '../scripts/release-version.mjs';
import { prepareAssets } from '../scripts/release-assets.mjs';

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'gibbon-release-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'src-tauri'));
  for (const p of ['package.json', 'package-lock.json', 'src-tauri/tauri.conf.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock']) {
    copyFileSync(new URL('../' + p, import.meta.url), join(root, p));
  }
  return root;
}

test('version bump updates all manifests and both lockfiles without changing dependencies', t => {
  const root = fixture(t);
  const before = JSON.parse(readFileSync(join(root, 'package-lock.json')));
  assert.equal(setVersion('9.0.0', root), '9.0.0');
  assert.equal(checkVersion('v9.0.0', root), '9.0.0');
  const after = JSON.parse(readFileSync(join(root, 'package-lock.json')));
  delete before.version; delete after.version;
  delete before.packages[''].version; delete after.packages[''].version;
  assert.deepEqual(after, before);
  assert.throws(() => checkVersion('v9.0.1', root));
  for (const value of ['9.0.0', '8.0.0', '09.1.0', '9.1.0-beta', undefined]) assert.throws(() => setVersion(value, root));
});

test('version drift blocks release and bump without writing files', t => {
  const root = fixture(t);
  const path = join(root, 'src-tauri/tauri.conf.json');
  const config = JSON.parse(readFileSync(path)); config.version = '0.0.0';
  writeFileSync(path, JSON.stringify(config));
  const before = readFileSync(join(root, 'package.json'), 'utf8');
  assert.throws(() => checkVersion(undefined, root));
  assert.throws(() => setVersion('9.0.0', root));
  assert.equal(readFileSync(join(root, 'package.json'), 'utf8'), before);
});

test('release assets require all three platforms and contain independently checkable hashes', t => {
  const root = mkdtempSync(join(tmpdir(), 'gibbon-assets-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const names = ['Agent Gibbon_9.0.0_aarch64.dmg', 'Agent Gibbon_9.0.0_x64.dmg', 'Agent Gibbon_9.0.0_x64-setup.exe'];
  assert.throws(() => prepareAssets(root, '9.0.0'));
  for (const name of names) writeFileSync(join(root, name), 'abc');
  writeFileSync(join(root, names[0]), '');
  assert.throws(() => prepareAssets(root, '9.0.0'));
  writeFileSync(join(root, names[0]), 'abc');
  writeFileSync(join(root, 'unexpected.exe'), 'abc');
  assert.throws(() => prepareAssets(root, '9.0.0'));
  rmSync(join(root, 'unexpected.exe'));
  assert.throws(() => prepareAssets(root, '9.0.1'));
  prepareAssets(root, '9.0.0');
  const sums = readFileSync(join(root, 'SHA256SUMS.txt'), 'utf8');
  assert.equal(sums, names.sort().map(name => `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  ${name}\n`).join(''));
});
