#!/usr/bin/env node
// Apply one cache/target policy to package scripts, without development identity in release builds.
import { spawn } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { devInstance, buildEnvironment, requireBuildDirSupport } from './dev-instance.mjs';
const instance = devInstance(fileURLToPath(new URL('..', import.meta.url)));
const [tool, ...args] = process.argv.slice(2);
if (!['cargo', 'tauri'].includes(tool)) {
  console.error('사용법: node scripts/build.mjs cargo|tauri [인자]');
  process.exit(1);
}
try { requireBuildDirSupport(); } catch (error) { console.error(error.message); process.exit(1); }
const command = tool === 'cargo' ? 'cargo' : process.execPath;
const commandArgs = tool === 'cargo' ? args : [join(instance.root, 'node_modules/@tauri-apps/cli/tauri.js'), ...args];
await import('./build-editor.mjs');
const child = spawn(command, commandArgs, { cwd: instance.root, env: buildEnvironment(instance), stdio: 'inherit' });
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal));
child.on('error', error => { console.error(error.message); process.exitCode = 1; });
child.on('exit', (code, signal) => { process.exitCode = code ?? (signal ? 1 : 0); });
