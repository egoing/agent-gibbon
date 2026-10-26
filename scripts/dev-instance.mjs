import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { realpathSync } from 'node:fs';
import { basename, join } from 'node:path';

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
  return { root, id, name, target: join(root, 'src-tauri/target'),
    binary: join(root, `src-tauri/target/debug/agent-gibbon${process.platform === 'win32' ? '.exe' : ''}`) };
}
export function devEnvironment(instance, inherited = process.env) {
  return { ...inherited, FLOATNOTE_INSTANCE: instance.id,
    FLOATNOTE_DEV_NAME: instance.name, CARGO_TARGET_DIR: instance.target };
}
