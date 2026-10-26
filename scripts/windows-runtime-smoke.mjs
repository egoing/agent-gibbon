// Built development worktrees only. Refuse pre-existing processes and clean up
// only the processes, note directories, and source files created by this run.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, realpathSync, rmSync, existsSync } from 'node:fs';
import { writeFileSync } from 'node:fs';
import { join, dirname, toNamespacedPath } from 'node:path';
import { fileURLToPath } from 'node:url';
import { devInstance } from './dev-instance.mjs';

assert.equal(process.platform, 'win32', 'Windows 전용 검증입니다.');
const root = realpathSync(fileURLToPath(new URL('..', import.meta.url)));
assert.ok(process.argv.length <= 3, '사용법: node scripts/windows-runtime-smoke.mjs [다른 워크트리 경로]');
const instances = [devInstance(root)];
if (process.argv[2]) instances.push(devInstance(process.argv[2]));
assert.equal(new Set(instances.map(i => i.id)).size, instances.length);
function processes(instance) {
  const result = spawnSync('powershell.exe', ['-NoProfile', '-Command',
    "@(Get-CimInstance Win32_Process -Filter \"name = 'agent-gibbon.exe'\" | Where-Object { $_.ExecutablePath -eq $env:GIBBON_SMOKE_BINARY } | ForEach-Object { $_.ProcessId }) | ConvertTo-Json -Compress"],
  { env: { ...process.env, GIBBON_SMOKE_BINARY: instance.binary }, encoding: 'utf8' });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
  const value = result.stdout.trim() ? JSON.parse(result.stdout) : [];
  return Array.isArray(value) ? value : [value];
}
for (const instance of instances) {
  assert.ok(existsSync(instance.binary), `먼저 dev:build를 실행하세요: ${instance.root}`);
  assert.deepEqual(processes(instance), [], `개발 앱이 이미 실행 중입니다: ${instance.root}`);
}
const owned = new Map();
const documents = new Map();
const temporary = mkdtempSync(join(root, '.windows-smoke-'));
const id = `win-runtime-${randomUUID()}`;
const identifier = JSON.parse(readFileSync(join(root, 'src-tauri/tauri.conf.json'), 'utf8')).identifier;
const noteRoot = join(process.env.APPDATA, identifier, 'notes');
function cli(instance, args, input, ok = true) {
  const result = spawnSync(process.execPath, [join(instance.root, 'scripts/dev.mjs'), 'cli', ...args],
    { cwd: dirname(root), encoding: 'utf8', input, timeout: 30000 });
  assert.ifError(result.error);
  assert.equal(result.status, ok ? 0 : 1, result.stderr + result.stdout);
  const value = JSON.parse(result.stdout);
  assert.equal(value.ok, ok, result.stdout);
  if (value.storage_path) {
    assert.ok([id, `${id}-restart`].includes(value.id));
    const expected = join(noteRoot, instance.id, 'docs', value.id, 'index.md');
    assert.equal(toNamespacedPath(value.storage_path).toLowerCase(), toNamespacedPath(expected).toLowerCase());
    documents.set(value.storage_path, value.id);
  }
  return value;
}
function own(instance) {
  const pids = processes(instance);
  assert.equal(pids.length, 1, `개발 프로세스를 식별할 수 없습니다: ${instance.binary}`);
  owned.set(instance, pids[0]);
}
async function stop(instance) {
  const pid = owned.get(instance);
  if (!pid) return;
  assert.deepEqual(processes(instance), [pid], '종료 전 실행 파일 경로와 PID 재확인 실패');
  process.kill(pid);
  for (let attempt = 0; attempt < 100; attempt++) {
    try { process.kill(pid, 0); } catch (error) {
      if (error.code !== 'ESRCH') throw error;
      owned.delete(instance);
      return;
    }
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`개발 프로세스 종료 대기 시간 초과: ${pid}`);
}
try {
  const tooLong = spawnSync(process.execPath, [join(root, 'scripts/dev.mjs'), 'cli', 'open', '--id', id], {
    cwd: dirname(root), encoding: 'utf8', timeout: 5000,
    env: { ...process.env, LOCALAPPDATA: join(temporary, '한'.repeat(50)) },
  });
  assert.ifError(tooLong.error);
  assert.equal(tooLong.status, 1, tooLong.stderr + tooLong.stdout);
  assert.match(JSON.parse(tooLong.stdout).error, /107바이트.*LOCALAPPDATA/);
  assert.deepEqual(processes(instances[0]), [], '잘못된 IPC 경로로 앱이 실행되었습니다.');
  for (const [index, instance] of instances.entries()) {
    let first;
    try {
      first = cli(instance, ['open', '--id', id, '--file', '-'], `# 한글 ${index}\n- [x] 표준입력\n`);
    } finally { own(instance); }
    assert.equal(first.instance, instance.id);
    // Closing cancels the deliberate cursor-directed entrance animation. It
    // must settle before geometry can be compared across content updates.
    cli(instance, ['close', '--id', id]);
    const before = cli(instance, ['inspect', '--id', id]);
    assert.equal(before.focused, false);
    assert.match(before.note.markdown, /한글/);
    cli(instance, ['update', '--id', id, '--markdown', '갱신 내용', '--color', '#dcecff']);
    const updated = cli(instance, ['inspect', '--id', id]);
    assert.deepEqual(updated.position, before.position);
    assert.equal(updated.focused, false);
    assert.equal(updated.note.color, '#dcecff');
    cli(instance, ['update', '--id', id, '--markdown', `숨긴 내용 ${index}`]);
    const hidden = cli(instance, ['inspect', '--id', id]);
    assert.equal(hidden.visible, false);
    assert.equal(hidden.note.markdown, `숨긴 내용 ${index}`);
    assert.match(readFileSync(hidden.storage_path, 'utf8'), /숨긴 내용/);
    cli(instance, ['update', '--id', id, '--color', 'bad'], undefined, false);
    assert.equal(cli(instance, ['inspect', '--id', id]).note.markdown, `숨긴 내용 ${index}`);
    const source = join(temporary, `한글 공백 ${index}.md`);
    writeFileSync(source, `# 연결 파일 ${index}\n`, 'utf8');
    cli(instance, ['update', '--id', id, '--file', source]);
    const linked = cli(instance, ['inspect', '--id', id]);
    assert.equal(linked.note.source_path, toNamespacedPath(source));
    assert.equal(linked.note.markdown, `# 연결 파일 ${index}\n`);
    cli(instance, ['update', '--id', id, '--title', '연결 보존']);
    assert.equal(cli(instance, ['inspect', '--id', id]).note.source_path, toNamespacedPath(source));
    assert.equal(cli(instance, ['--serve']).already_running, true);
    assert.equal(processes(instance).length, 1);
  }
  const first = instances[0];
  const peerBefore = instances[1] && cli(instances[1], ['inspect', '--id', id]);
  await stop(first);
  cli(first, ['inspect', '--id', id], undefined, false);
  try { cli(first, ['open', '--id', `${id}-restart`, '--markdown', '재시작 확인']); }
  finally { own(first); }
  const restored = cli(first, ['inspect', '--id', id]);
  assert.equal(restored.visible, false);
  assert.equal(restored.note.markdown, '# 연결 파일 0\n');
  assert.equal(restored.note.title, '연결 보존');
  assert.equal(restored.note.color, '#dcecff');
  assert.equal(readFileSync(restored.note.source_path, 'utf8'), '# 연결 파일 0\n');
  cli(first, ['update', '--id', id, '--markdown', '연결 해제']);
  assert.equal(cli(first, ['inspect', '--id', id]).note.source_path, null);
  if (peerBefore) assert.deepEqual(cli(instances[1], ['inspect', '--id', id]), peerBefore);
  console.log(JSON.stringify({ ok: true, instances: instances.map(i => i.id), checks: [
    'long UTF-8 IPC path fails before launch', 'Korean stdin', 'focus and position preservation', 'hidden update', 'persistence',
    'invalid input preserves state', 'Korean and spaced source path', 'source link preservation and detachment',
    'duplicate launch lock', 'restart restoration', ...(peerBefore ? ['same-ID instance and restart isolation'] : [])] }));
} catch (error) {
  console.error(error);
  throw error;
} finally {
  for (const instance of [...owned.keys()]) await stop(instance);
  // All registered paths came from this run's unique IDs. Stop before touching
  // storage so no live app retains a deleted note or recreates its document.
  for (const [path, noteId] of documents) {
    assert.equal(dirname(path).split(/[\\/]/).at(-1), noteId);
    rmSync(dirname(path), { recursive: true, force: true });
  }
  rmSync(temporary, { recursive: true, force: true, maxRetries: 20, retryDelay: 100 });
}
