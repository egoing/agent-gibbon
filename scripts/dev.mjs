#!/usr/bin/env node
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, realpathSync } from 'node:fs';
import { devInstance, devEnvironment } from './dev-instance.mjs';

const instance = devInstance(fileURLToPath(new URL('..', import.meta.url)));
const [mode, ...args] = process.argv.slice(2);
const env = devEnvironment(instance);
let command, commandArgs;
if (mode === 'info') {
  console.log(JSON.stringify(instance));
  process.exit(0);
} else if (mode === 'cli') {
  if (!existsSync(instance.binary)) {
    console.log(JSON.stringify({ ok: false, error: '개발 실행 파일이 없습니다. npm run dev:build를 실행하세요.' }));
    process.exit(1);
  }
  const probe = spawnSync(instance.binary, ['--instance-info'], { env, encoding: 'utf8' });
  let valid = false;
  try {
    const info = JSON.parse(probe.stdout);
    valid = probe.status === 0 && info.instance === instance.id
      && realpathSync(info.manifest_dir) === realpathSync(join(instance.root, 'src-tauri'));
  } catch { /* Old or foreign binaries must never reach the installed socket. */ }
  if (!valid) {
    console.log(JSON.stringify({ ok: false, error: '현재 워크트리의 개발 바이너리가 아닙니다. npm run dev:build로 다시 빌드하세요.' }));
    process.exit(1);
  }
  command = instance.binary;
  commandArgs = args;
} else if (mode === 'build') {
  command = 'cargo';
  commandArgs = ['build', '--manifest-path', `${instance.root}/src-tauri/Cargo.toml`, '--target-dir', instance.target];
} else if (mode === 'dev') {
  // Do not permit options that silently change the binary/profile/target used by CLI.
  if (args.some(arg => arg !== '--no-watch')) {
    console.error('개발 실행은 --no-watch만 추가로 지원합니다.');
    process.exit(1);
  }
  command = process.execPath;
  commandArgs = [join(instance.root, 'node_modules/@tauri-apps/cli/tauri.js'), 'dev', '--no-dev-server', ...args, '--', '--target-dir', instance.target];
} else {
  console.error('사용법: node scripts/dev.mjs dev|build|cli|info');
  process.exit(1);
}
const child = spawn(command, commandArgs, { cwd: mode === 'cli' ? process.cwd() : instance.root, env, stdio: 'inherit' });
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal));
child.on('error', error => { console.error(error.message); process.exitCode = 1; });
child.on('exit', (code, signal) => { process.exitCode = code ?? (signal ? 1 : 0); });
