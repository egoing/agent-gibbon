import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const semver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const paths = ['package.json', 'package-lock.json', 'src-tauri/tauri.conf.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock'];
const cargoPattern = /(^\[package\]\s*\nname = "agent-gibbon"\s*\nversion = ")([^"]+)(")/m;
const lockPattern = /(^\[\[package\]\]\s*\nname = "agent-gibbon"\s*\nversion = ")([^"]+)(")/m;

export function versions(base = root) {
  const texts = paths.map(p => readFileSync(resolve(base, p), 'utf8'));
  const pkg = JSON.parse(texts[0]);
  const lock = JSON.parse(texts[1]);
  const config = JSON.parse(texts[2]);
  return { texts, values: [pkg.version, lock.version, lock.packages[''].version, config.version,
    texts[3].match(cargoPattern)?.[2], texts[4].match(lockPattern)?.[2]] };
}

export function checkVersion(tag, base = root) {
  const { values } = versions(base);
  const version = values[0];
  if (!semver.test(version) || values.some(v => v !== version)) throw new Error('npm·Cargo·Tauri와 lockfile 버전이 일치해야 합니다.');
  if (tag !== undefined && tag !== `v${version}`) throw new Error(`태그는 v${version}이어야 합니다.`);
  return version;
}

export function setVersion(version, base = root) {
  if (!semver.test(version)) throw new Error('안정 버전을 X.Y.Z 형식으로 지정하세요.');
  const current = checkVersion(undefined, base);
  const a = version.split('.').map(Number), b = current.split('.').map(Number);
  const first = a.findIndex((v, i) => v !== b[i]);
  if (first < 0 || a[first] < b[first]) throw new Error('현재보다 높은 버전이어야 합니다.');
  const { texts } = versions(base);
  const updated = texts.map((text, i) => {
    if (i === 3) return text.replace(cargoPattern, `$1${version}$3`);
    if (i === 4) return text.replace(lockPattern, `$1${version}$3`);
    const json = JSON.parse(text);
    json.version = version;
    if (i === 1) json.packages[''].version = version;
    return JSON.stringify(json, null, 2) + '\n';
  });
  updated.forEach((text, i) => writeFileSync(resolve(base, paths[i]), text));
  return checkVersion(undefined, base);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const [command, value] = process.argv.slice(2);
    if (command === 'check') console.log(checkVersion(value));
    else if (command === 'set') console.log(setVersion(value));
    else throw new Error('사용법: release-version.mjs check [vX.Y.Z] | set X.Y.Z');
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
