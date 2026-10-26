import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { readFileSync } from 'node:fs';
const launcher = fileURLToPath(new URL('./dev.mjs', import.meta.url));
const installedBinary = process.argv[2];
const ids = [`win-smoke-${process.pid}`, `win-peer-${process.pid}`];
function cli(args, input, ok = true) {
  console.log('CLI', args[0], args[2] ?? '');
  const result = spawnSync(installedBinary ?? process.execPath, installedBinary ? args : [launcher, 'cli', ...args], { encoding: 'utf8', input, timeout: 30000 });
  assert.ifError(result.error);
  assert.equal(result.status, ok ? 0 : 1, result.stderr + result.stdout);
  const value = JSON.parse(result.stdout);
  assert.equal(value.ok, ok, result.stdout);
  return value;
}
try {
  const opened = cli(['open', '--id', ids[0], '--file', '-'], '# Windows 한글\n- [x] CLI 확인\n');
  const first = cli(['inspect', '--id', ids[0]]);
  assert.equal(first.visible, true);
  assert.equal(first.focused, false);
  assert.match(first.note.markdown, /한글/);
  cli(['open', '--id', ids[1], '--markdown', '독립 메모']);
  cli(['update', '--id', ids[0], '--markdown', '# 갱신\n- [ ] 다음 단계', '--color', '#dcecff']);
  const updated = cli(['inspect', '--id', ids[0]]);
  assert.deepEqual(updated.position, first.position);
  assert.equal(updated.focused, false);
  assert.equal(updated.note.color, '#dcecff');
  assert.equal(cli(['inspect', '--id', ids[1]]).note.markdown, '독립 메모');
  cli(['close', '--id', ids[0]]);
  cli(['update', '--id', ids[0], '--markdown', '숨긴 상태 갱신']);
  const hidden = cli(['inspect', '--id', ids[0]]);
  assert.equal(hidden.visible, false);
  assert.equal(hidden.note.markdown, '숨긴 상태 갱신');
  assert.match(readFileSync(hidden.storage_path, 'utf8'), /숨긴 상태 갱신/);
  cli(['update', '--id', ids[0], '--color', 'bad'], undefined, false);
  assert.equal(cli(['inspect', '--id', ids[0]]).note.markdown, '숨긴 상태 갱신');
  cli(['open', '--id', ids[0]]);
  assert.equal(cli(['inspect', '--id', ids[0]]).visible, true);
  console.log(JSON.stringify({ ok: true, checks: ['Korean Markdown', 'two independent notes', 'focus and position preservation', 'hidden update', 'persistence file', 'invalid input preserves state', 'reopen'], instance: opened.instance, ids }));
} finally {
  for (const id of ids) { try { cli(['close', '--id', id]); } catch (error) { console.error('Cleanup:', error.message); } }
}
