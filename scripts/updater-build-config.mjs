import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export function createUpdaterBuildConfig(publicKey) {
  if (typeof publicKey !== 'string' || publicKey.trim() === '') {
    throw new Error('TAURI_UPDATER_PUBLIC_KEY is required for updater builds');
  }

  return {
    bundle: { createUpdaterArtifacts: true },
    plugins: { updater: { pubkey: publicKey } },
  };
}

export function writeUpdaterBuildConfig(outputPath, publicKey) {
  const config = createUpdaterBuildConfig(publicKey);
  writeFileSync(resolve(outputPath), `${JSON.stringify(config)}\n`, { mode: 0o600 });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const outputPath = process.argv[2];
    if (!outputPath) throw new Error('Usage: node scripts/updater-build-config.mjs <output-path>');
    writeUpdaterBuildConfig(outputPath, process.env.TAURI_UPDATER_PUBLIC_KEY);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
