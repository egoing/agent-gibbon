import { createHash } from 'node:crypto';
import { realpathSync } from 'node:fs';
import { basename, join } from 'node:path';

// Canonical absolute paths make symlink entry points and arbitrary cwd agree.
export function devInstance(root) {
  root = realpathSync(root);
  const id = `dev-${createHash('sha256').update(root).digest('hex').slice(0, 24)}`;
  return { root, id, name: basename(root), target: join(root, 'src-tauri/target'),
    binary: join(root, 'src-tauri/target/debug/floatnote') };
}
export function devEnvironment(instance, inherited = process.env) {
  return { ...inherited, FLOATNOTE_INSTANCE: instance.id,
    FLOATNOTE_DEV_NAME: instance.name, CARGO_TARGET_DIR: instance.target };
}
