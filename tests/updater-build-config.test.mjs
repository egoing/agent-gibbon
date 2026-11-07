import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createUpdaterBuildConfig, writeUpdaterBuildConfig } from '../scripts/updater-build-config.mjs';

test('updater build overlay enables signed artifacts and embeds the exact public key', t => {
  const publicKey = 'untrusted comment: minisign public key\nRWQxMjM0NTY3ODkwYWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo=';
  const config = createUpdaterBuildConfig(publicKey);

  assert.deepEqual(config, {
    bundle: { createUpdaterArtifacts: true },
    plugins: { updater: { pubkey: publicKey } },
  });

  const directory = mkdtempSync(join(tmpdir(), 'gibbon-updater-config-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const outputPath = join(directory, 'tauri.conf.json');
  writeUpdaterBuildConfig(outputPath, publicKey);
  assert.deepEqual(JSON.parse(readFileSync(outputPath, 'utf8')), config);
});

test('updater build overlay fails closed when public key is missing', () => {
  assert.throws(() => createUpdaterBuildConfig(undefined), /TAURI_UPDATER_PUBLIC_KEY is required/);
  assert.throws(() => createUpdaterBuildConfig(' \n '), /TAURI_UPDATER_PUBLIC_KEY is required/);
});
