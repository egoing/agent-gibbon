import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { realpathSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';

// Canonical absolute paths make symlink entry points and arbitrary cwd agree.
export function devInstance(root) {
  root = realpathSync(root);
  const id = `dev-${createHash('sha256').update(root).digest('hex').slice(0, 24)}`;
  const branch = spawnSync('git', ['-C', root, 'symbolic-ref', '--quiet', '--short', 'HEAD'], { encoding: 'utf8' });
  let name = branch.status === 0 ? branch.stdout.trim() : '';
  if (!name) {
    const head = spawnSync('git', ['-C', root, 'rev-parse', '--short', 'HEAD'], { encoding: 'utf8' });
    name = head.status === 0 ? `detached@${head.stdout.trim()}` : basename(root);
  }
  const common = spawnSync('git', ['-C', root, 'rev-parse', '--path-format=absolute', '--git-common-dir'], { encoding: 'utf8' });
  // Linked worktrees share this directory; independent repositories do not.
  // Source archives without Git retain the ordinary local Cargo layout.
  const defaultBuildDir = common.status === 0
    ? join(realpathSync(common.stdout.trim()), 'agent-gibbon', 'build-cache')
    : join(root, 'src-tauri/target');
  const buildDir = resolve(root, process.env.CARGO_BUILD_BUILD_DIR || defaultBuildDir);
  return { root, id, name, buildDir, target: join(root, 'src-tauri/target'),
    binary: join(root, `src-tauri/target/debug/agent-gibbon${process.platform === 'win32' ? '.exe' : ''}`) };
}
export function buildEnvironment(instance, inherited = process.env) {
  return { ...inherited, CARGO_TARGET_DIR: instance.target,
    CARGO_BUILD_BUILD_DIR: resolve(instance.root, inherited.CARGO_BUILD_BUILD_DIR || instance.buildDir) };
}
export function devEnvironment(instance, inherited = process.env) {
  return { ...buildEnvironment(instance, inherited), FLOATNOTE_INSTANCE: instance.id,
    FLOATNOTE_DEV_NAME: instance.name };
}
export function requireBuildDirSupport() {
  const result = spawnSync('cargo', ['--version'], { encoding: 'utf8' });
  const version = result.stdout?.match(/^cargo (\d+)\.(\d+)\./);
  if (result.status !== 0 || !version || (+version[1] === 1 && +version[2] < 91) || +version[1] < 1) {
    throw new Error('중간 캐시 분리에는 Cargo 1.91 이상이 필요합니다. rustup update stable로 갱신하세요.');
  }
}
