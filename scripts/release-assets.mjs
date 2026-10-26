import { createHash } from 'node:crypto';
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export function prepareAssets(directory, version) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) throw new Error('Invalid release version');
  const expected = [`Agent Gibbon_${version}_aarch64.dmg`, `Agent Gibbon_${version}_x64.dmg`, `Agent Gibbon_${version}_x64-setup.exe`].sort();
  const found = readdirSync(directory).sort();
  if (JSON.stringify(found) !== JSON.stringify(expected)) throw new Error('Expected exactly two macOS DMGs and one Windows installer for this version');
  const lines = expected.map(name => {
    const path = resolve(directory, name);
    if (!statSync(path).isFile() || statSync(path).size === 0) throw new Error(`Empty or invalid installer: ${name}`);
    return `${createHash('sha256').update(readFileSync(path)).digest('hex')}  ${name}`;
  });
  writeFileSync(resolve(directory, 'SHA256SUMS.txt'), lines.join('\n') + '\n');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { prepareAssets(...process.argv.slice(2)); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}
