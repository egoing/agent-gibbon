import test from 'node:test';
import { generateKeyPairSync, sign, createHash } from 'node:crypto';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, existsSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { prepareAssets } from '../scripts/release-assets.mjs';

// Signed updater releases must fail closed before a manifest/checksum can be published.
// Platform selection is deliberately not assumed by these negative cases.
function installers(t) {
  const directory = mkdtempSync(join(tmpdir(), 'gibbon-updater-assets-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  for (const name of ['Agent Gibbon_9.0.0_aarch64.dmg', 'Agent Gibbon_9.0.0_x64.dmg', 'Agent Gibbon_9.0.0_x64-setup.exe']) {
    writeFileSync(join(directory, name), 'fixture installer');
  }
  return directory;
}

const updaterFiles = version => [
  `Agent Gibbon_${version}_darwin-aarch64.app.tar.gz`,
  `Agent Gibbon_${version}_darwin-x86_64.app.tar.gz`,
  `Agent Gibbon_${version}_aarch64.dmg`,
  `Agent Gibbon_${version}_x64.dmg`,
  `Agent Gibbon_${version}_x64-setup.exe`,
];

function signedAssets(directory, version) {
  for (const name of updaterFiles(version)) writeFileSync(join(directory, name), `payload ${name}`);
  for (const [target, filename] of [
    ['darwin-aarch64', `Agent Gibbon_${version}_darwin-aarch64.app.tar.gz`],
    ['darwin-x86_64', `Agent Gibbon_${version}_darwin-x86_64.app.tar.gz`],
    ['windows-x86_64', `Agent Gibbon_${version}_x64-setup.exe`],
  ]) {
    const {privateKey, publicKey} = generateKeyPairSync('ed25519');
    const id = Buffer.alloc(8, 1);
    const pub = Buffer.concat([Buffer.from('Ed'),id,publicKey.export({format:'der',type:'spki'}).subarray(-32)]);
    const signature = sign(null, createHash('blake2b512').update(readFileSync(join(directory,filename))).digest(),privateKey);
    const trusted = `timestamp:1791649051\tfile:${filename}\tversion:${version}`;
    const global = sign(null,Buffer.concat([signature,Buffer.from(trusted)]),privateKey);
    writeFileSync(join(directory,`${target}.pub`),Buffer.from(`untrusted comment: test public key\n${pub.toString('base64')}`).toString('base64'));
    writeFileSync(join(directory,`${filename}.sig`),Buffer.from(`untrusted comment: signature from tauri secret key\n${Buffer.concat([Buffer.from('ED'),id,signature]).toString('base64')}\ntrusted comment: ${trusted}\n${global.toString('base64')}\n`).toString('base64'));
  }
}

test('updater release rejects installer-only assets without signed update bundles', t => {
  const directory = installers(t);
  assert.throws(() => prepareAssets(directory, '9.0.0', { updater: true }), /updat|signatur/i);
  assert.equal(existsSync(join(directory, 'latest.json')), false);
  assert.equal(existsSync(join(directory, 'SHA256SUMS.txt')), false);
});

test('failed updater preparation preserves an existing manifest instead of publishing unsigned assets', t => {
  const directory = installers(t);
  const manifest = join(directory, 'latest.json');
  const original = '{"version":"8.0.0"}\n';
  writeFileSync(manifest, original);
  assert.throws(() => prepareAssets(directory, '9.0.0', { updater: true }));
  assert.equal(readFileSync(manifest, 'utf8'), original);
});

test('updater preparation emits a target-specific manifest with encoded release URLs and signed versions', t => {
  const directory = installers(t);
  signedAssets(directory, '9.0.0');

  prepareAssets(directory, '9.0.0', { updater: true, repository: 'egoing/agent-gibbon' });

  const manifest = JSON.parse(readFileSync(join(directory, 'latest.json'), 'utf8'));
  assert.equal(manifest.version, '9.0.0');
  assert.equal(manifest.notes, '');
  assert.deepEqual(Object.keys(manifest.platforms).sort(), ['darwin-aarch64', 'darwin-x86_64', 'windows-x86_64']);
  assert.equal(manifest.platforms['darwin-aarch64'].url, 'https://github.com/egoing/agent-gibbon/releases/download/v9.0.0/Agent%20Gibbon_9.0.0_darwin-aarch64.app.tar.gz');
  assert.equal(manifest.platforms['darwin-aarch64'].signature, readFileSync(join(directory, 'Agent Gibbon_9.0.0_darwin-aarch64.app.tar.gz.sig'), 'utf8'));
  assert.match(Buffer.from(manifest.platforms['darwin-aarch64'].signature, 'base64').toString('utf8'), /version:9\.0\.0/);
  assert.equal(manifest.platforms['windows-x86_64'].url, 'https://github.com/egoing/agent-gibbon/releases/download/v9.0.0/Agent%20Gibbon_9.0.0_x64-setup.exe');
  assert.equal(manifest.platforms['windows-x86_64'].signature, readFileSync(join(directory, 'Agent Gibbon_9.0.0_x64-setup.exe.sig'), 'utf8'));
  assert.equal(readFileSync(join(directory, 'SHA256SUMS.txt'), 'utf8').trim().split('\n').length, 11);
});

test('updater preparation rejects signatures that do not bind the published stable version', t => {
  const directory = installers(t);
  signedAssets(directory, '9.0.0');
  const signaturePath = join(directory, 'Agent Gibbon_9.0.0_darwin-aarch64.app.tar.gz.sig');
  const decodedSignature = Buffer.from(readFileSync(signaturePath, 'utf8'), 'base64').toString('utf8');
  writeFileSync(signaturePath, Buffer.from(decodedSignature.replace('version:9.0.0', 'version:8.0.0')).toString('base64'));
  const manifest = join(directory, 'latest.json');
  const original = '{"version":"8.0.0"}\n';
  writeFileSync(manifest, original);

  assert.throws(() => prepareAssets(directory, '9.0.0', { updater: true, repository: 'egoing/agent-gibbon' }), /signature/i);
  assert.equal(readFileSync(manifest, 'utf8'), original);
  assert.equal(existsSync(join(directory, 'SHA256SUMS.txt')), false);
});

test('updater preparation only accepts stable semver versions', t => {
  const directory = installers(t);
  signedAssets(directory, '9.0.0-rc.1');

  assert.throws(() => prepareAssets(directory, '9.0.0-rc.1', { updater: true, repository: 'egoing/agent-gibbon' }), /version/i);
  assert.equal(existsSync(join(directory, 'latest.json')), false);
});

test('damaged payload, wrong signing key and swapped platforms never publish a manifest', t => {
  for (const mutation of ['payload', 'key', 'platform']) {
    const directory = installers(t); signedAssets(directory, '9.0.0');
    const arm = join(directory,'Agent Gibbon_9.0.0_darwin-aarch64.app.tar.gz');
    const intel = join(directory,'Agent Gibbon_9.0.0_darwin-x86_64.app.tar.gz');
    if (mutation === 'payload') writeFileSync(arm,'tampered');
    if (mutation === 'key') writeFileSync(join(directory,'darwin-aarch64.pub'), readFileSync(join(directory,'windows-x86_64.pub')));
    if (mutation === 'platform') {
      writeFileSync(arm,readFileSync(intel)); writeFileSync(`${arm}.sig`,readFileSync(`${intel}.sig`));
      writeFileSync(join(directory,'darwin-aarch64.pub'),readFileSync(join(directory,'darwin-x86_64.pub')));
    }
    assert.throws(()=>prepareAssets(directory,'9.0.0',{updater:true}),/signature/);
    assert.equal(existsSync(join(directory,'latest.json')),false);
  }
});

test('Tauri CLI signatures verify with the release verifier without private keys', t => {
  const directory = installers(t);
  const fixture = new URL('./fixtures/updater/', import.meta.url);
  for (const [target, name] of [
    ['darwin-aarch64','Agent Gibbon_9.0.0_darwin-aarch64.app.tar.gz'],
    ['darwin-x86_64','Agent Gibbon_9.0.0_darwin-x86_64.app.tar.gz'],
    ['windows-x86_64','Agent Gibbon_9.0.0_x64-setup.exe'],
  ]) {
    writeFileSync(join(directory,name),readFileSync(new URL('payload',fixture)));
    writeFileSync(join(directory,`${name}.sig`),readFileSync(new URL(`${target}.sig`,fixture)));
    writeFileSync(join(directory,`${target}.pub`),readFileSync(new URL('public.key',fixture)));
  }
  prepareAssets(directory,'9.0.0',{updater:true,expectedPublicKey:readFileSync(new URL('public.key',fixture),'utf8')});
  assert.equal(JSON.parse(readFileSync(join(directory,'latest.json'),'utf8')).version,'9.0.0');
  assert.throws(()=>prepareAssets(directory,'9.0.0',{updater:true,expectedPublicKey:'wrong release trust root'}),/trust root/);
});
