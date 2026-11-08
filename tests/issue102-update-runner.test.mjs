import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, existsSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  archiveUpdaterState,
  candidateManifest,
  createFeedServer,
  recordMarker,
  summarizeDiagnostics,
  supportsSessionRestore,
  validateCandidate,
  verificationStorePath,
} from '../scripts/issue102-update-runner.mjs';
import { createHash } from 'node:crypto';

function fixtureDir(t) {
  const dir = mkdtempSync(join(tmpdir(), 'issue102-runner-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}

const fixture = new URL('./fixtures/updater/', import.meta.url);
const version = '9.0.0';
const target = 'darwin-x86_64';
const fileName = 'Agent.Gibbon_9.0.0_darwin-x86_64.app.tar.gz';
const payload = readFileSync(new URL('payload', fixture));
const signature = readFileSync(new URL('darwin-x86_64.sig', fixture), 'utf8').trim();
const publicKey = readFileSync(new URL('public.key', fixture), 'utf8').trim();
const hash = createHash('sha256').update(payload).digest('hex');

test('session restore marker detection requires the new save_for_restart implementation and caller', t => {
  const source = fixtureDir(t);
  mkdirSync(join(source, 'src-tauri/src'), { recursive: true });
  writeFileSync(join(source, 'src-tauri/src/update_restore.rs'), 'pub fn save_for_restart() {}');
  writeFileSync(join(source, 'src-tauri/src/updater.rs'), 'update_restore::save_for_restart();');
  assert.equal(supportsSessionRestore(source), true);
  writeFileSync(join(source, 'src-tauri/src/updater.rs'), 'update_restore::save();');
  assert.equal(supportsSessionRestore(source), false);
});

test('candidate manifest adds the restore marker only when source or explicit option enables it', t => {
  const endpoint = 'http://127.0.0.1:49231/latest.json';
  const base = candidateManifest({ version, target, endpoint, signature, marker: 'off' });
  assert.equal(Object.hasOwn(base, 'gibbon_session_restore'), false);
  const explicit = candidateManifest({ version, target, endpoint, signature, marker: 'on' });
  assert.equal(explicit.gibbon_session_restore, 1);
  assert.throws(() => candidateManifest({ version, target, endpoint: 'http://0.0.0.0:49231/latest.json', signature }), /127\.0\.0\.1/);
});

test('candidate validation binds stable version, target filename, SHA-256 and minisign signature', t => {
  const root = fixtureDir(t);
  mkdirSync(join(root, 'v9.0.0'), { recursive: true });
  writeFileSync(join(root, 'v9.0.0', fileName), payload);
  const manifest = {
    version,
    platforms: { [target]: { url: 'http://127.0.0.1:49231/v9.0.0/' + fileName, signature } },
  };
  const result = validateCandidate({
    manifest, assetRoot: root, endpoint: 'http://127.0.0.1:49231/latest.json',
    target, expectedVersion: version, expectedSha256: hash, publicKey, expectedSessionRestore: false,
  });
  assert.deepEqual(
    { version: result.version, target: result.target, fileName: result.fileName, sha256: result.sha256, bytes: result.bytes },
    { version, target, fileName, sha256: hash, bytes: payload.length },
  );
  assert.throws(() => validateCandidate({
    manifest, assetRoot: root, endpoint: 'http://127.0.0.1:49231/latest.json',
    target, expectedVersion: version, expectedSha256: '0'.repeat(64), publicKey, expectedSessionRestore: false,
  }), /SHA-256/);
  assert.throws(() => validateCandidate({
    manifest: { ...manifest, gibbon_session_restore: 1 }, assetRoot: root,
    endpoint: 'http://127.0.0.1:49231/latest.json', target, expectedVersion: version,
    expectedSha256: hash, publicKey, expectedSessionRestore: false,
  }), /gibbon_session_restore/);
});

test('verification feed binds only to loopback and serves normal, offline, tampered and interrupted responses', async t => {
  const root = fixtureDir(t);
  mkdirSync(join(root, 'v9.0.0'), { recursive: true });
  writeFileSync(join(root, 'latest.json'), JSON.stringify({
    version, platforms: { [target]: { url: 'http://127.0.0.1/v9.0.0/' + fileName, signature } },
  }));
  writeFileSync(join(root, 'v9.0.0', fileName), payload);
  assert.throws(() => createFeedServer({ root, host: '0.0.0.0' }), /127\.0\.0\.1/);

  const get = async url => fetch(url);
  const normal = await createFeedServer({ root });
  t.after(() => normal.close());
  assert.equal(new URL(normal.endpoint).hostname, '127.0.0.1');
  assert.equal((await get(normal.endpoint)).status, 200);
  assert.equal(await (await get(new URL('v9.0.0/' + fileName, normal.endpoint))).arrayBuffer().then(value => value.byteLength), payload.length);
  assert.equal((await get(new URL('../etc/passwd', normal.endpoint))).status, 404);

  const offline = await createFeedServer({ root, fault: 'offline' });
  t.after(() => offline.close());
  assert.equal((await get(offline.endpoint)).status, 503);

  const tampered = await createFeedServer({ root, fault: 'tamper-signature' });
  t.after(() => tampered.close());
  const changed = await (await get(tampered.endpoint)).json();
  assert.notEqual(changed.platforms[target].signature, signature);

  const interrupted = await createFeedServer({ root, fault: 'disconnect-download' });
  t.after(() => interrupted.close());
  const interruptedResponse = await get(new URL('v9.0.0/' + fileName, interrupted.endpoint));
  assert.equal(interruptedResponse.status, 200);
  await assert.rejects(interruptedResponse.arrayBuffer());

  const delayed = await createFeedServer({ root, fault: 'delay-download', delayMs: 10 });
  t.after(() => delayed.close());
  const start = Date.now();
  assert.equal((await get(new URL('v9.0.0/' + fileName, delayed.endpoint))).status, 200);
  assert.ok(Date.now() - start >= 8);
});

test('update cache and restore files archive without moving note content', t => {
  const runRoot = fixtureDir(t);
  const docs = verificationStorePath(runRoot);
  mkdirSync(docs, { recursive: true });
  writeFileSync(join(docs, '.update-cache'), 'verified bytes');
  writeFileSync(join(docs, '.last-update-attempt.json'), '{}');
  writeFileSync(join(docs, '.update-notice.json'), '{}');
  writeFileSync(join(docs, 'note.md'), 'keep');
  const archived = archiveUpdaterState(runRoot, 'before-empty-cache');
  assert.equal(archived.moved.length, 3);
  assert.equal(existsSync(join(docs, '.update-cache')), false);
  assert.equal(readFileSync(join(docs, 'note.md'), 'utf8'), 'keep');
  assert.equal(readFileSync(join(archived.archive, '.update-cache'), 'utf8'), 'verified bytes');
  assert.throws(() => archiveUpdaterState(runRoot, '../escape'), /단계 이름/);
});

test('event markers and diagnostics summaries omit note contents and unrelated fields', t => {
  const events = join(fixtureDir(t), 'events.jsonl');
  recordMarker(events, 'baseline-saved', { version: '0.2.11', sha256: hash, note: 'stable state' });
  const entry = JSON.parse(readFileSync(events, 'utf8'));
  assert.equal(entry.version, '0.2.11');
  assert.equal(entry.sha256, hash);
  assert.throws(() => recordMarker(events, 'arbitrary-data', {}), /알 수 없는/);
  assert.deepEqual(summarizeDiagnostics({
    instance: 'dev-test',
    updater: { enabled: true, status: { current_version: '0.2.11', version: '0.2.12', phase: 'ready' }, idle_threshold_seconds: 30, check_seconds: 60, retry_seconds: 5 },
    note: { markdown: 'private' },
  }), {
    instance: 'dev-test', enabled: true, current_version: '0.2.11', target_version: '0.2.12',
    phase: 'ready', idle_threshold_seconds: 30, check_seconds: 60, retry_seconds: 5,
    last_window_reports: null,
  });
});
