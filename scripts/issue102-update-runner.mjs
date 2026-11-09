#!/usr/bin/env node
import { createHash } from 'node:crypto';
import {
  appendFileSync, copyFileSync, cpSync, existsSync, lstatSync, mkdirSync, readFileSync,
  readdirSync, realpathSync, readlinkSync, renameSync, rmSync, symlinkSync, writeFileSync,
} from 'node:fs';
import { createServer } from 'node:http';
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { tmpdir } from 'node:os';
import { spawn, spawnSync } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import { verifyUpdaterSignature } from './release-assets.mjs';

export const VERIFICATION_INSTANCE = 'dev-001020010200102001020010';
export const FAULT_MODES = new Set(['none', 'offline', 'disconnect-download', 'tamper-signature', 'delay-download']);
export const MARKERS = new Set([
  'baseline-saved', 'app-start', 'read-note-opened', 'edit-entered', 'terminal-started',
  'last-input', 'candidate-published', 'candidate-downloaded', 'install-request-auto',
  'install-request-manual', 'install-phase', 'network-failure', 'signature-rejected',
  'restart', 'stable-after-restart', 'snapshot-before', 'snapshot-after',
]);
export const PLATFORM_ASSETS = Object.freeze({
  'darwin-aarch64': version => 'Agent.Gibbon_' + version + '_darwin-aarch64.app.tar.gz',
  'darwin-x86_64': version => 'Agent.Gibbon_' + version + '_darwin-x86_64.app.tar.gz',
  'windows-x86_64': version => 'Agent.Gibbon_' + version + '_x64-setup.exe',
});

function assertVersion(version) {
  if (typeof version !== 'string' || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
    throw new Error('안정 버전 형식이 아닙니다: ' + version);
  }
}
function requiredString(value, name) {
  if (typeof value !== 'string' || value.trim() === '') throw new Error(name + ' 값이 필요합니다');
  return value.trim();
}
function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}
function safeJoin(root, requestPath) {
  const base = resolve(root);
  const decoded = decodeURIComponent(requestPath);
  const candidate = resolve(base, '.' + (decoded.startsWith('/') ? decoded : '/' + decoded));
  const rel = relative(base, candidate);
  if (rel === '') return candidate;
  if (rel.startsWith('..' + sep) || rel === '..' || isAbsolute(rel)) throw new Error('피드 경로가 루트를 벗어납니다');
  return candidate;
}
function assertLoopbackUrl(value) {
  const url = new URL(value);
  if (url.protocol !== 'http:' || url.hostname !== '127.0.0.1' || url.username || url.password) {
    throw new Error('검증 피드는 http://127.0.0.1 에만 연결할 수 있습니다');
  }
  return url;
}

export function supportsSessionRestore(sourceRoot) {
  if (!sourceRoot) return false;
  const restorePath = resolve(sourceRoot, 'src-tauri/src/update_restore.rs');
  const updaterPath = resolve(sourceRoot, 'src-tauri/src/updater.rs');
  if (!existsSync(restorePath) || !existsSync(updaterPath)) return false;
  const restore = readFileSync(restorePath, 'utf8');
  const updater = readFileSync(updaterPath, 'utf8');
  return /\bpub\s+fn\s+save_for_restart\s*\(/.test(restore)
    && /\bupdate_restore::save_for_restart\s*\(/.test(updater);
}

export function candidateManifest({ version, target, endpoint, signature, sourceRoot, marker = 'auto' }) {
  assertVersion(version);
  const fileName = PLATFORM_ASSETS[target]?.(version);
  if (!fileName) throw new Error('지원하지 않는 업데이트 대상: ' + target);
  const base = assertLoopbackUrl(endpoint);
  if (!['auto', 'on', 'off'].includes(marker)) throw new Error('marker는 auto, on, off 중 하나여야 합니다');
  const restoreEnabled = marker === 'on' || (marker === 'auto' && supportsSessionRestore(sourceRoot));
  const result = {
    version,
    notes: '격리 검증 후보',
    platforms: { [target]: { url: new URL('v' + version + '/' + fileName, base).href, signature: requiredString(signature, '서명') } },
  };
  if (restoreEnabled) result.gibbon_session_restore = 1;
  return result;
}

export function validateCandidate({
  manifest, assetRoot, endpoint, target, expectedVersion, expectedSha256, publicKey, expectedSessionRestore,
}) {
  assertVersion(expectedVersion);
  if (manifest?.version !== expectedVersion) throw new Error('manifest 버전이 예상 버전과 다릅니다');
  if (expectedSessionRestore === true && manifest.gibbon_session_restore !== 1) {
    throw new Error('후보 manifest에 gibbon_session_restore: 1이 없습니다');
  }
  if (expectedSessionRestore === false && Object.hasOwn(manifest, 'gibbon_session_restore')) {
    throw new Error('복원 기능이 없는 후보에 gibbon_session_restore가 있습니다');
  }
  const descriptor = manifest.platforms?.[target];
  if (!descriptor) throw new Error('manifest에 ' + target + ' 항목이 없습니다');
  const expectedFile = PLATFORM_ASSETS[target]?.(expectedVersion);
  if (!expectedFile) throw new Error('지원하지 않는 업데이트 대상: ' + target);
  const base = assertLoopbackUrl(endpoint);
  const expectedUrl = new URL('v' + expectedVersion + '/' + expectedFile, base);
  const actualUrl = new URL(descriptor.url);
  if (actualUrl.href !== expectedUrl.href) throw new Error('manifest URL이 버전·플랫폼 경로와 다릅니다');
  const assetPath = safeJoin(assetRoot, actualUrl.pathname);
  const bytes = readFileSync(assetPath);
  const actualSha256 = sha256(bytes);
  if (actualSha256 !== requiredString(expectedSha256, '예상 SHA-256').toLowerCase()) {
    throw new Error('후보 파일의 SHA-256이 예상 값과 다릅니다');
  }
  verifyUpdaterSignature(descriptor.signature, requiredString(publicKey, '공개 키'), bytes, expectedVersion, expectedFile);
  return {
    version: expectedVersion, target, fileName: expectedFile, sha256: actualSha256,
    bytes: bytes.length, sessionRestore: manifest.gibbon_session_restore === 1,
  };
}

function atomicWrite(path, contents) {
  mkdirSync(dirname(path), { recursive: true });
  const temp = path + '.' + process.pid + '.tmp';
  writeFileSync(temp, contents, { flag: 'wx', mode: 0o600 });
  renameSync(temp, path);
}
function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

export function recordMarker(eventsPath, event, details = {}) {
  if (!MARKERS.has(event)) throw new Error('알 수 없는 실행 기록 항목: ' + event);
  if (details.version !== undefined) assertVersion(details.version);
  const entry = {
    timestamp: new Date().toISOString(), epoch_ms: Date.now(), event,
    ...(details.version ? { version: details.version } : {}),
    ...(details.sha256 ? { sha256: details.sha256 } : {}),
    ...(details.phase ? { phase: String(details.phase).slice(0, 32) } : {}),
    ...(details.note ? { note: String(details.note).slice(0, 120) } : {}),
  };
  mkdirSync(dirname(resolve(eventsPath)), { recursive: true });
  appendFileSync(eventsPath, JSON.stringify(entry) + '\n', { mode: 0o600 });
  return entry;
}

export function verificationStorePath(runRoot) {
  return join(resolve(runRoot), 'store', 'notes', VERIFICATION_INSTANCE, 'docs');
}
export function archiveUpdaterState(runRoot, stage) {
  if (!/^[A-Za-z0-9_-]{1,48}$/.test(stage)) throw new Error('보관 단계 이름이 올바르지 않습니다');
  const docs = verificationStorePath(runRoot);
  const archive = join(resolve(runRoot), 'store-archive', stage);
  mkdirSync(archive, { recursive: true });
  const moved = [];
  for (const name of ['.update-cache', '.last-update-attempt.json', '.update-notice.json']) {
    const source = join(docs, name);
    if (!existsSync(source)) continue;
    if (lstatSync(source).isSymbolicLink()) throw new Error('심볼릭 링크 상태 파일은 보관하지 않습니다: ' + name);
    const destination = join(archive, name);
    if (existsSync(destination)) throw new Error('보관 파일이 이미 있습니다: ' + destination);
    renameSync(source, destination);
    moved.push(name);
  }
  return { docs, archive, moved };
}

export function createFeedServer({ root, host = '127.0.0.1', port = 0, fault = 'none', delayMs = 5000, logPath }) {
  if (host !== '127.0.0.1') throw new Error('검증 피드는 127.0.0.1에만 바인딩할 수 있습니다');
  if (!FAULT_MODES.has(fault)) throw new Error('지원하지 않는 오류 주입 모드: ' + fault);
  const feedRoot = realpathSync(root);
  const log = entry => {
    if (!logPath) return;
    mkdirSync(dirname(resolve(logPath)), { recursive: true });
    appendFileSync(logPath, JSON.stringify({ timestamp: new Date().toISOString(), ...entry }) + '\n', { mode: 0o600 });
  };
  const server = createServer((request, response) => {
    void (async () => {
      const method = request.method || 'GET';
      const pathname = new URL(request.url || '/', 'http://127.0.0.1').pathname;
      if (!['GET', 'HEAD'].includes(method)) {
        response.writeHead(405).end();
        log({ method, path: pathname, status: 405, bytes: 0, fault });
        return;
      }
      if (fault === 'offline') {
        response.writeHead(503, { 'cache-control': 'no-store' }).end();
        log({ method, path: pathname, status: 503, bytes: 0, fault });
        return;
      }
      let file;
      try {
        file = realpathSync(safeJoin(feedRoot, pathname));
        const rel = relative(feedRoot, file);
        if (!rel || rel.startsWith('..' + sep) || rel === '..' || !lstatSync(file).isFile()) throw new Error('not a file');
      } catch {
        response.writeHead(404, { 'cache-control': 'no-store' }).end();
        log({ method, path: pathname, status: 404, bytes: 0, fault });
        return;
      }
      if (fault === 'delay-download' && pathname.startsWith('/v')) {
        await delay(Math.max(0, Math.min(Number(delayMs) || 0, 120000)));
      }
      let bytes = readFileSync(file);
      if (fault === 'tamper-signature' && pathname === '/latest.json') {
        try {
          const manifest = JSON.parse(bytes.toString('utf8'));
          const entry = Object.values(manifest.platforms || {})[0];
          if (!entry?.signature || entry.signature.length < 8) throw new Error('signature missing');
          const index = entry.signature.length - 5;
          entry.signature = entry.signature.slice(0, index)
            + (entry.signature[index] === 'A' ? 'B' : 'A')
            + entry.signature.slice(index + 1);
          bytes = Buffer.from(JSON.stringify(manifest) + '\n');
        } catch {
          response.writeHead(500, { 'cache-control': 'no-store' }).end();
          log({ method, path: pathname, status: 500, bytes: 0, fault });
          return;
        }
      }
      response.writeHead(200, {
        'content-type': pathname.endsWith('.json') ? 'application/json' : 'application/octet-stream',
        'content-length': bytes.length,
        'cache-control': 'no-store',
      });
      if (method === 'HEAD') {
        response.end();
        log({ method, path: pathname, status: 200, bytes: 0, fault });
        return;
      }
      if (fault === 'disconnect-download' && pathname.startsWith('/v') && bytes.length > 1) {
        const partial = Math.max(1, Math.floor(bytes.length / 3));
        response.write(bytes.subarray(0, partial));
        setImmediate(() => response.destroy());
        log({ method, path: pathname, status: 200, bytes: partial, fault });
        return;
      }
      response.end(bytes);
      log({ method, path: pathname, status: 200, bytes: bytes.length, fault });
    })().catch(() => {
      if (!response.headersSent) response.writeHead(500).end();
      else response.destroy();
    });
  });
  return new Promise((resolvePromise, reject) => {
    server.once('error', reject);
    server.listen(port, host, () => {
      server.removeListener('error', reject);
      const address = server.address();
      resolvePromise({
        server, endpoint: 'http://' + host + ':' + address.port + '/latest.json',
        close: () => new Promise((resolveClose, rejectClose) => server.close(error => error ? rejectClose(error) : resolveClose())),
      });
    });
  });
}

function makeRunDirectory(runRoot, { target, previousVersion, candidateVersion, runId }) {
  const root = resolve(runRoot);
  assertVersion(previousVersion);
  assertVersion(candidateVersion);
  if (!PLATFORM_ASSETS[target]) throw new Error('지원하지 않는 업데이트 대상: ' + target);
  if (existsSync(join(root, 'run.json'))) throw new Error('이미 실행기 회차가 초기화되어 있습니다');
  mkdirSync(root, { recursive: true });
  const metadata = {
    schema: 1, run_id: runId || 'issue102-' + Date.now(), created_at: new Date().toISOString(),
    target, previous_version: previousVersion, candidate_version: candidateVersion,
    artifact_root: join(root, 'artifacts'), feed_root: join(root, 'feed'),
    store_root: join(root, 'store'), events: join(root, 'events.jsonl'), server_log: join(root, 'http.jsonl'),
  };
  for (const key of ['artifact_root', 'feed_root', 'store_root']) mkdirSync(metadata[key], { recursive: true, mode: 0o700 });
  atomicWrite(join(root, 'run.json'), JSON.stringify(metadata, null, 2) + '\n');
  return metadata;
}
function loadRun(runRoot) {
  const root = canonicalPath(runRoot);
  const metadata = readJson(join(root, 'run.json'));
  if (metadata.schema !== 1) throw new Error('실행 메타데이터 스키마가 다릅니다');
  for (const key of ['artifact_root', 'feed_root', 'store_root', 'events', 'server_log']) {
    if (typeof metadata[key] === 'string') metadata[key] = canonicalPath(metadata[key]);
  }
  return { root, metadata };
}
export function fingerprintTree(rootPath) {
  const root = resolve(rootPath);
  const entries = [];
  let bytes = 0;
  const visit = (path, relativePath) => {
    const info = lstatSync(path);
    if (info.isSymbolicLink()) {
      entries.push({ path: relativePath, type: 'link', target: readlinkSync(path) });
      return;
    }
    if (info.isDirectory()) {
      entries.push({ path: relativePath, type: 'directory' });
      for (const name of readdirSync(path).sort()) {
        visit(join(path, name), relativePath ? relativePath + '/' + name : name);
      }
      return;
    }
    if (!info.isFile()) throw new Error('산출물에 지원하지 않는 파일 형식이 있습니다: ' + relativePath);
    const digest = sha256(readFileSync(path));
    bytes += info.size;
    entries.push({ path: relativePath, type: 'file', bytes: info.size, sha256: digest });
  };
  visit(root, '');
  const digest = sha256(Buffer.from(JSON.stringify(entries)));
  return { sha256: digest, bytes, entries: entries.length };
}
function runLogged(program, args, options, logPath, label) {
  const result = spawnSync(program, args, { ...options, encoding: 'utf8' });
  const output = (result.stdout || '') + (result.stderr || '');
  appendFileSync(logPath, '--- ' + label + ' (exit ' + (result.status ?? 'error') + ') ---\n' + output + '\n');
  if (result.error || result.status !== 0) {
    throw new Error(label + ' 실패(exit ' + (result.status ?? 'error') + '). 상세는 build.log를 확인하세요.');
  }
  return output;
}
function copyBuildArtifact(source, destination, platform) {
  if (existsSync(destination)) throw new Error('기존 빌드 산출물을 덮어쓰지 않습니다: ' + destination);
  mkdirSync(dirname(destination), { recursive: true });
  if (platform === 'darwin') {
    const result = spawnSync('ditto', [source, destination], { encoding: 'utf8' });
    if (result.status !== 0) throw new Error('macOS app 빌드 출력을 분리 보관하지 못했습니다');
  } else {
    copyFileSync(source, destination);
  }
}
function expectedHost(target) {
  if (target.startsWith('darwin-')) {
    const arch = target.endsWith('aarch64') ? 'arm64' : 'x64';
    if (process.platform !== 'darwin' || process.arch !== arch) {
      throw new Error('대상 ' + target + '은 같은 macOS CPU 호스트에서 빌드해야 합니다');
    }
    return 'darwin';
  }
  if (process.platform !== 'win32' || process.arch !== 'x64') {
    throw new Error('Windows x64 검증 후보는 Windows x64 호스트에서 빌드해야 합니다');
  }
  return 'win32';
}
export function buildPair({ runDir, sourceRoot, nodeModules, cargoTargetDir, endpoint, publicKeyFile, candidateStandard = false }) {
  const { root, metadata } = loadRun(runDir);
  const host = expectedHost(metadata.target);
  const source = resolve(requiredString(sourceRoot, '--source'));
  if (!existsSync(source) || isWithin(source, root) || isWithin(root, source)) {
    throw new Error('소스와 회차 폴더는 서로 분리되어 있어야 합니다');
  }
  const copy = join(root, 'source');
  if (existsSync(copy)) throw new Error('소스 복사본이 이미 있습니다. 새 회차 폴더를 사용하세요');
  const targetDir = resolve(requiredString(cargoTargetDir, '--cargo-target-dir'));
  if (!isAbsolute(targetDir) || isWithin(copy, targetDir)) throw new Error('Cargo release 캐시는 절대 경로여야 하며 소스 복사본 밖에 있어야 합니다');
  const endpointUrl = assertLoopbackUrl(requiredString(endpoint, '--endpoint'));
  if (new URL(endpointUrl.href).pathname !== '/latest.json') throw new Error('endpoint 경로는 /latest.json이어야 합니다');
  const publicKey = readFileSync(resolve(requiredString(publicKeyFile, '--public-key-file')), 'utf8').trim();
  cpSync(source, copy, {
    recursive: true,
    filter: path => {
      const rel = relative(source, path).split(/[\\/]/);
      const base = rel.at(-1);
      if (base === '.git' || base === 'node_modules' || base === 'target') return false;
      if (base === '.env' || (base.startsWith('.env.') && base !== '.env.example')) return false;
      return !rel.includes('src-tauri/target');
    },
  });
  const gitPointer = join(source, '.git');
  if (existsSync(gitPointer) && lstatSync(gitPointer).isFile()) {
    const content = readFileSync(gitPointer, 'utf8');
    if (content.startsWith('gitdir: ')) writeFileSync(join(copy, '.git'), content);
  }
  const modulesSource = resolve(nodeModules || join(source, 'node_modules'));
  const modulesDestination = join(copy, 'node_modules');
  if (existsSync(modulesSource)) {
    const type = process.platform === 'win32' ? 'junction' : 'dir';
    // The linked dependency tree stays read-only from the runner's perspective.
    awaitSymlink(modulesSource, modulesDestination, type);
  } else {
    runLogged('npm', ['ci', '--no-audit', '--no-fund'], { cwd: copy }, join(root, 'build.log'), 'npm ci');
  }
  const tauriCli = join(modulesDestination, '@tauri-apps', 'cli', 'tauri.js');
  if (!existsSync(tauriCli)) throw new Error('Tauri CLI를 찾을 수 없습니다: ' + tauriCli);
  const configPath = join(copy, 'verify.json');
  const config = {
    identifier: 'org.egoing.floatnote.verification',
    plugins: { updater: { pubkey: publicKey, dangerousInsecureTransportProtocol: true } },
  };
  writeFileSync(configPath, JSON.stringify(config, null, 2) + '\n', { mode: 0o600 });
  const logPath = join(root, 'build.log');
  const versionsTool = join(copy, 'scripts', 'release-version.mjs');
  const versionsToolArg = relative(copy, versionsTool);
  const sourceVersion = runLogged(process.execPath, [versionsToolArg, 'check'], { cwd: copy }, logPath, 'check source version').trim().split(/\r?\n/).at(-1);
  if (sourceVersion !== metadata.previous_version) {
    throw new Error('소스 버전 ' + sourceVersion + '가 이전 버전 ' + metadata.previous_version + '과 다릅니다');
  }
  const profiles = [
    { name: 'previous-fast', version: metadata.previous_version, standard: false },
    { name: 'previous-standard', version: metadata.previous_version, standard: true },
    { name: 'candidate', version: metadata.candidate_version, standard: candidateStandard },
  ];
  const appBundleRoot = host === 'darwin'
    ? join(targetDir, 'release', 'bundle', 'macos')
    : join(targetDir, 'release', 'bundle', 'nsis');
  const artifactMap = {};
  for (const profile of profiles) {
    if (profile.name === 'candidate') {
      runLogged(process.execPath, [versionsToolArg, 'set', profile.version], { cwd: copy }, logPath, 'set candidate version');
      runLogged(process.execPath, [versionsToolArg, 'check'], { cwd: copy }, logPath, 'check candidate version');
    }
    const env = {
      ...process.env,
      CARGO_TARGET_DIR: targetDir,
      GIBBON_VERIFICATION_UPDATE_URL: endpointUrl.href,
      TAURI_UPDATER_PUBLIC_KEY: publicKey,
    };
    if (profile.standard) env.GIBBON_VERIFICATION_STANDARD_TIMING = '1';
    else delete env.GIBBON_VERIFICATION_STANDARD_TIMING;
    const bundleArg = host === 'darwin' ? 'app' : 'nsis';
    runLogged(process.execPath, [
      relative(copy, tauriCli), 'build', '--features', 'verification-updater', '--config', configPath, '--bundles', bundleArg,
    ], { cwd: copy, env }, logPath, 'build ' + profile.name + ' ' + profile.version);
    let built;
    let destination;
    if (host === 'darwin') {
      built = join(appBundleRoot, 'Agent Gibbon.app');
      destination = join(metadata.artifact_root, profile.name + '.app');
    } else {
      const matches = readdirSync(appBundleRoot).filter(name => name.includes(profile.version) && name.endsWith('_x64-setup.exe'));
      if (matches.length !== 1) throw new Error('NSIS 후보를 하나로 특정하지 못했습니다: ' + matches.join(', '));
      built = join(appBundleRoot, matches[0]);
      destination = join(metadata.artifact_root, profile.name + '-installer.exe');
    }
    if (!existsSync(built)) throw new Error('Tauri 빌드 산출물이 없습니다: ' + built);
    copyBuildArtifact(built, destination, host);
    artifactMap[profile.name] = {
      version: profile.version, standard_timing: profile.standard,
      path: destination, ...fingerprintTree(destination),
    };
    atomicWrite(join(metadata.artifact_root, 'profiles.json'), JSON.stringify(artifactMap, null, 2) + '\n');
  }
  const sourceFingerprint = fingerprintTree(copy);
  const revisionResult = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: copy, encoding: 'utf8' });
  const buildInfo = {
    source: copy, source_revision: revisionResult.status === 0 ? revisionResult.stdout.trim() : 'unknown',
    source_version: sourceVersion, target: metadata.target, host,
    cargo_target_dir: targetDir, endpoint: endpointUrl.href, built_at: new Date().toISOString(),
    artifacts: artifactMap, source_fingerprint: sourceFingerprint,
  };
  atomicWrite(join(root, 'build.json'), JSON.stringify(buildInfo, null, 2) + '\n');
  return buildInfo;
}
function awaitSymlink(targetPath, linkPath, type) {
  // Node's directory symlink mode is 'junction' on Windows and 'dir' on macOS.
  symlinkSync(targetPath, linkPath, type);
}
function isWithin(root, file) {
  const rel = relative(canonicalPath(root), canonicalPath(file));
  return rel === '' || (!rel.startsWith('..' + sep) && rel !== '..' && !isAbsolute(rel));
}
function canonicalPath(path) {
  const absolute = resolve(path);
  try {
    return realpathSync(absolute);
  } catch {
    return absolute;
  }
}
export function validateProcessRecord(record, root, storeRoot) {
  if (!Number.isSafeInteger(record?.pid) || record.pid <= 0) throw new Error('process.json PID가 올바르지 않습니다');
  if (typeof record.executable !== 'string' || !isAbsolute(record.executable) || !isWithin(root, record.executable)) {
    throw new Error('process.json 실행 파일이 회차 디렉터리 밖입니다');
  }
  if (typeof record.store_root !== 'string'
      || canonicalPath(record.store_root) !== canonicalPath(storeRoot)
      || !isWithin(root, record.store_root)) {
    throw new Error('process.json Store가 현재 회차와 일치하지 않습니다');
  }
  if (!['darwin', 'win32'].includes(record.platform)) throw new Error('process.json 플랫폼이 올바르지 않습니다');
  return record;
}
export function validateDiagnosticsOwner(diagnostics, expectedStoreRoot) {
  if (diagnostics?.ok !== true || diagnostics.instance !== VERIFICATION_INSTANCE) {
    throw new Error('진단 응답 인스턴스가 검증 인스턴스와 일치하지 않습니다');
  }
  if (typeof diagnostics.verification_store_root !== 'string'
      || canonicalPath(diagnostics.verification_store_root) !== canonicalPath(expectedStoreRoot)) {
    throw new Error('진단 응답 Store가 현재 회차와 일치하지 않습니다');
  }
  return diagnostics;
}
function executableFor(appPath, platform) {
  if (platform === 'darwin') {
    const app = resolve(appPath);
    if (!app.endsWith('.app')) throw new Error('macOS 실행 경로는 .app 번들이어야 합니다');
    return canonicalPath(join(app, 'Contents', 'MacOS', 'agent-gibbon'));
  }
  if (platform === 'win32') {
    if (extname(appPath).toLowerCase() !== '.exe') throw new Error('Windows 실행 경로는 설치된 agent-gibbon.exe여야 합니다');
    return canonicalPath(appPath);
  }
  throw new Error('macOS 또는 Windows에서만 검증 앱을 실행할 수 있습니다');
}
export function launchVerificationApp({ runRoot, appPath, platform = process.platform, headless = false }) {
  const { root, metadata } = loadRun(runRoot);
  if (!isWithin(root, appPath)) throw new Error('앱 경로는 전용 실행기 디렉터리 안이어야 합니다');
  const executable = executableFor(appPath, platform);
  if (!existsSync(executable)) throw new Error('검증 실행 파일이 없습니다: ' + executable);
  const pidFile = join(root, 'process.json');
  if (existsSync(pidFile)) throw new Error('이 회차의 프로세스 기록이 이미 있습니다');
  mkdirSync(metadata.store_root, { recursive: true, mode: 0o700 });
  const child = spawn(executable, headless ? ['--serve'] : [], {
    cwd: dirname(executable), detached: true, stdio: 'ignore',
    env: { ...process.env, AGENT_GIBBON_VERIFICATION_STORE_ROOT: metadata.store_root },
  });
  if (!child.pid) throw new Error('검증 앱 프로세스를 시작하지 못했습니다');
  child.unref();
  atomicWrite(pidFile, JSON.stringify({
    pid: child.pid, executable, started_at: new Date().toISOString(),
    store_root: metadata.store_root, platform,
  }, null, 2) + '\n');
  recordMarker(metadata.events, 'app-start', { version: metadata.previous_version });
  return { pid: child.pid, executable, storeRoot: metadata.store_root, pidFile };
}
function processExecutable(pid, platform) {
  if (platform === 'darwin') {
    const result = spawnSync('lsof', ['-a', '-p', String(pid), '-d', 'txt', '-Fn'], { encoding: 'utf8' });
    return result.status === 0 ? result.stdout.split(/\r?\n/).find(line => line.startsWith('n/'))?.slice(1) || null : null;
  }
  if (platform === 'win32') {
    const script = '$p=Get-CimInstance Win32_Process -Filter \'ProcessId=' + Number(pid) + '\'; if ($p) { $p.ExecutablePath }';
    const result = spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' });
    return result.status === 0 ? result.stdout.trim() || null : null;
  }
  return null;
}
function processIdsForExecutable(executable, platform) {
  if (platform === 'darwin') {
    const pattern = '^' + executable.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '([[:space:]].*)?$';
    const result = spawnSync('pgrep', ['-f', pattern], { encoding: 'utf8' });
    if (result.status === 1) return [];
    if (result.status !== 0) throw new Error('검증 프로세스 목록을 확인하지 못했습니다');
    return result.stdout.split(/\r?\n/).filter(Boolean).map(Number).filter(Number.isSafeInteger);
  }
  if (platform === 'win32') {
    const quoted = "'" + executable.replace(/'/g, "''") + "'";
    const script = '$expected = [System.IO.Path]::GetFullPath(' + quoted + '); Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -and [System.IO.Path]::GetFullPath($_.ExecutablePath) -eq $expected } | ForEach-Object { $_.ProcessId }';
    const result = spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' });
    if (result.status !== 0) throw new Error('검증 프로세스 목록을 확인하지 못했습니다');
    return result.stdout.split(/\r?\n/).filter(Boolean).map(Number).filter(Number.isSafeInteger);
  }
  throw new Error('지원하지 않는 검증 플랫폼입니다');
}
function readOwnedDiagnostics(executable, storeRoot) {
  const result = spawnSync(executable, ['diagnostics'], {
    encoding: 'utf8', timeout: 15000,
    env: { ...process.env, AGENT_GIBBON_VERIFICATION_STORE_ROOT: storeRoot },
  });
  if (result.error || result.status !== 0) throw new Error('검증 앱 diagnostics를 조회하지 못했습니다');
  return validateDiagnosticsOwner(JSON.parse(result.stdout), storeRoot);
}
function resolveActiveProcess(root, metadata) {
  const processPath = join(root, 'process.json');
  const record = validateProcessRecord(readJson(processPath), root, metadata.store_root);
  const pids = new Set([record.pid, ...processIdsForExecutable(canonicalPath(record.executable), record.platform)]);
  const matches = [];
  for (const pid of pids) {
    const actual = processExecutable(pid, record.platform);
    if (!actual || canonicalPath(actual) !== canonicalPath(record.executable)) continue;
    try {
      const diagnostics = readOwnedDiagnostics(record.executable, record.store_root);
      matches.push({ pid, diagnostics });
    } catch {
      // An executable match without matching instance and Store evidence is not owned by this run.
    }
  }
  if (matches.length !== 1) {
    throw new Error('실행 중인 검증 앱을 PID·실행 파일·인스턴스·회차 Store로 유일하게 식별하지 못했습니다');
  }
  const [{ pid, diagnostics }] = matches;
  if (pid !== record.pid) {
    const updated = { ...record, pid, restarted_at: new Date().toISOString() };
    atomicWrite(processPath, JSON.stringify(updated, null, 2) + '\n');
    recordMarker(metadata.events, 'restart', {
      version: diagnostics.updater?.status?.current_version || metadata.candidate_version,
      note: 'PID가 바뀐 뒤 실행 파일·인스턴스·회차 Store를 재확인했습니다',
    });
    return { record: updated, diagnostics };
  }
  return { record, diagnostics };
}
export async function stopOwnedApp(runRoot, platform = process.platform) {
  const { root, metadata } = loadRun(runRoot);
  const record = validateProcessRecord(readJson(join(root, 'process.json')), root, metadata.store_root);
  const actual = processExecutable(record.pid, platform);
  if (!actual || canonicalPath(actual) !== canonicalPath(record.executable)) {
    throw new Error('실행 파일 출처가 다르므로 이 PID를 종료하지 않았습니다');
  }
  if (platform === 'win32') {
    const result = spawnSync('taskkill.exe', ['/PID', String(record.pid), '/T'], { encoding: 'utf8' });
    if (result.status !== 0) throw new Error('검증 앱을 종료하지 못했습니다');
  } else {
    process.kill(record.pid, 'SIGTERM');
  }
  for (let i = 0; i < 50; i += 1) {
    if (!processExecutable(record.pid, platform)) return { stopped: true, pid: record.pid };
    await delay(100);
  }
  throw new Error('검증 앱이 종료되지 않았습니다');
}
export function summarizeDiagnostics(value) {
  const updater = value?.updater || {};
  const status = updater.status || {};
  return {
    instance: value?.instance ?? null, enabled: updater.enabled === true,
    current_version: status.current_version ?? null, target_version: status.version ?? null,
    phase: status.phase ?? null, idle_threshold_seconds: updater.idle_threshold_seconds ?? null,
    idle_seconds: updater.idle_seconds ?? null,
    check_seconds: updater.check_seconds ?? null, retry_seconds: updater.retry_seconds ?? null,
    last_check_failure: updater.last_check_failure ?? null,
    last_window_reports: updater.last_window_reports ?? null,
  };
}

async function prepareCandidate(args) {
  const { metadata } = loadRun(args.runDir);
  const target = args.target || metadata.target;
  const version = args.version || metadata.candidate_version;
  assertVersion(version);
  if (version !== metadata.candidate_version || target !== metadata.target) throw new Error('후보 버전·플랫폼이 회차와 다릅니다');
  const artifact = resolve(requiredString(args.artifact, '--artifact'));
  const publicKeyPath = resolve(requiredString(args.publicKeyFile, '--public-key-file'));
  const privateKeyPath = resolve(requiredString(args.privateKeyFile, '--private-key-file'));
  if (!existsSync(artifact) || !existsSync(publicKeyPath) || !existsSync(privateKeyPath)) throw new Error('산출물 또는 서명 키 파일이 없습니다');
  if (process.platform !== 'win32' && (lstatSync(privateKeyPath).mode & 0o077) !== 0) {
    throw new Error('개인 키 파일 권한은 소유자 전용이어야 합니다');
  }
  const publicKey = readFileSync(publicKeyPath, 'utf8').trim();
  const fileName = PLATFORM_ASSETS[target](version);
  const versionDir = join(metadata.feed_root, 'v' + version);
  mkdirSync(versionDir, { recursive: true, mode: 0o700 });
  const assetPath = join(versionDir, fileName);
  if (existsSync(assetPath) || existsSync(assetPath + '.sig') || existsSync(join(metadata.feed_root, 'candidate.json'))) {
    throw new Error('산출물이 이미 있습니다. 새 회차 디렉터리를 사용하세요');
  }
  let temporary;
  try {
    if (target.startsWith('darwin-')) {
      if (process.platform !== 'darwin') throw new Error('macOS .app는 macOS 호스트에서만 포장할 수 있습니다');
      temporary = join(tmpdir(), 'issue102-package-' + process.pid + '-' + Date.now());
      mkdirSync(temporary, { recursive: true });
      const copied = spawnSync('ditto', [artifact, join(temporary, 'Agent Gibbon.app')], { encoding: 'utf8' });
      if (copied.status !== 0) throw new Error('검증 app 번들을 staging하지 못했습니다');
      const archived = spawnSync('tar', ['-czf', assetPath, '-C', temporary, 'Agent Gibbon.app'], {
        encoding: 'utf8', env: { ...process.env, COPYFILE_DISABLE: '1' },
      });
      if (archived.status !== 0) throw new Error('macOS updater 아카이브를 만들지 못했습니다');
    } else {
      if (process.platform !== 'win32') throw new Error('Windows updater 자산은 Windows 호스트에서 준비해야 합니다');
      copyFileSync(artifact, assetPath);
    }
    const signer = resolve(requiredString(args.tauriCli, '--tauri-cli'));
    const password = args.passwordFile ? readFileSync(resolve(args.passwordFile), 'utf8').replace(/\r?\n$/, '') : '';
    const signed = spawnSync(process.execPath, [signer, 'signer', 'sign', '--app-version', version, assetPath], {
      encoding: 'utf8', stdio: ['ignore', 'ignore', 'ignore'],
      env: { ...process.env, TAURI_SIGNING_PRIVATE_KEY_PATH: privateKeyPath, TAURI_SIGNING_PRIVATE_KEY_PASSWORD: password },
    });
    if (signed.error || signed.status !== 0) throw new Error('Tauri 서명이 실패했습니다. 키 내용은 출력하지 않았습니다');
    const signature = readFileSync(assetPath + '.sig', 'utf8').trim();
    const manifest = candidateManifest({
      version, target, endpoint: requiredString(args.endpoint, '--endpoint'), signature,
      sourceRoot: args.source ? resolve(args.source) : undefined,
      marker: args.sessionRestoreMarker || 'auto',
    });
    const result = validateCandidate({
      manifest, assetRoot: metadata.feed_root, endpoint: args.endpoint, target,
      expectedVersion: version, expectedSha256: args.expectedSha256 || sha256(readFileSync(assetPath)),
      publicKey, expectedSessionRestore: manifest.gibbon_session_restore === 1,
    });
    atomicWrite(join(metadata.feed_root, 'candidate.json'), JSON.stringify(manifest, null, 2) + '\n');
    atomicWrite(join(metadata.feed_root, 'candidate.sha256'), result.sha256 + '\n');
    return { ...result, feedRoot: metadata.feed_root, manifestPath: join(metadata.feed_root, 'candidate.json') };
  } catch (error) {
    if (existsSync(assetPath)) rmSync(assetPath, { force: true });
    if (existsSync(assetPath + '.sig')) rmSync(assetPath + '.sig', { force: true });
    throw error;
  } finally {
    if (temporary && existsSync(temporary)) rmSync(temporary, { recursive: true, force: true });
  }
}

function verifyRunCandidate(runRoot, publicKeyFile, expectedSessionRestore) {
  const { metadata } = loadRun(runRoot);
  const manifest = readJson(join(metadata.feed_root, 'candidate.json'));
  const expectedSha256 = readFileSync(join(metadata.feed_root, 'candidate.sha256'), 'utf8').trim();
  const endpoint = new URL(manifest.platforms[metadata.target].url);
  return validateCandidate({
    manifest, assetRoot: metadata.feed_root,
    endpoint: 'http://127.0.0.1:' + endpoint.port + '/latest.json',
    target: metadata.target, expectedVersion: metadata.candidate_version, expectedSha256,
    publicKey: readFileSync(publicKeyFile, 'utf8').trim(), expectedSessionRestore,
  });
}
function publishCandidate({ runDir, publicKeyFile, expectedSessionRestore }) {
  const { root, metadata } = loadRun(runDir);
  const candidate = verifyRunCandidate(root, resolve(publicKeyFile), expectedSessionRestore);
  const events = existsSync(metadata.events)
    ? readFileSync(metadata.events, 'utf8').split(/\r?\n/).filter(Boolean).map(line => JSON.parse(line))
    : [];
  if (!events.some(event => event.event === 'baseline-saved' && event.version === metadata.previous_version)) {
    throw new Error('이전 버전의 baseline-saved 기록 전에는 후보를 공개하지 않습니다');
  }
  const latest = join(metadata.feed_root, 'latest.json');
  if (existsSync(latest)) throw new Error('latest.json이 이미 있습니다. 새 회차 디렉터리를 사용하세요');
  atomicWrite(latest, JSON.stringify(readJson(join(metadata.feed_root, 'candidate.json')), null, 2) + '\n');
  recordMarker(metadata.events, 'candidate-published', { version: candidate.version, sha256: candidate.sha256 });
  return { latest, ...candidate };
}

function usage() {
  return [
    'Issue 102 isolated update runner',
    '  init --run-dir PATH --target darwin-x86_64|darwin-aarch64|windows-x86_64 --previous-version X.Y.Z --candidate-version X.Y.Z',
    '  package-candidate --run-dir PATH --artifact PATH --public-key-file PATH --private-key-file PATH --tauri-cli PATH --endpoint http://127.0.0.1:PORT/latest.json [--source PATH] [--session-restore-marker auto|on|off]',
    '  mark --run-dir PATH --event NAME [--version X.Y.Z] [--sha256 HEX] [--note TEXT]',
    '  publish --run-dir PATH --public-key-file PATH [--session-restore]',
    '  serve --run-dir PATH [--port PORT] [--fault none|offline|disconnect-download|tamper-signature|delay-download] [--delay-ms MS]',
    '  verify --run-dir PATH --public-key-file PATH [--session-restore]',
    '  archive-state --run-dir PATH --stage NAME',
    '  launch --run-dir PATH --app PATH [--platform darwin|win32] [--headless]',
    '  status --run-dir PATH',
    '  stop --run-dir PATH [--platform darwin|win32]',
    '',
  ].join('\n');
}
function parseArgs(argv) {
  const [command, ...rest] = argv;
  const args = {};
  for (let i = 0; i < rest.length; i += 1) {
    const key = rest[i];
    if (!key.startsWith('--')) throw new Error('알 수 없는 인수: ' + key);
    const name = key.slice(2).replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
    if (name === 'sessionRestore' || name === 'headless') {
      args[name] = true;
      continue;
    }
    const value = rest[i + 1];
    if (value === undefined || value.startsWith('--')) throw new Error(key + ' 값이 필요합니다');
    args[name] = value;
    i += 1;
  }
  return { command, args };
}
async function main(argv) {
  const { command, args } = parseArgs(argv);
  if (!command || command === 'help' || command === '--help') {
    process.stdout.write(usage());
    return;
  }
  if (command === 'init') {
    const metadata = makeRunDirectory(requiredString(args.runDir, '--run-dir'), {
      target: requiredString(args.target, '--target'),
      previousVersion: requiredString(args.previousVersion, '--previous-version'),
      candidateVersion: requiredString(args.candidateVersion, '--candidate-version'),
      runId: args.runId,
    });
    process.stdout.write(JSON.stringify(metadata, null, 2) + '\n');
    return;
  }
  if (command === 'package-candidate') {
    process.stdout.write(JSON.stringify(await prepareCandidate(args), null, 2) + '\n');
    return;
  }
  if (command === 'mark') {
    const { metadata } = loadRun(requiredString(args.runDir, '--run-dir'));
    process.stdout.write(JSON.stringify(recordMarker(metadata.events, requiredString(args.event, '--event'), args)) + '\n');
    return;
  }
  if (command === 'verify') {
    const result = verifyRunCandidate(args.runDir, resolve(requiredString(args.publicKeyFile, '--public-key-file')), args.sessionRestore);
    process.stdout.write(JSON.stringify({ ok: true, ...result }) + '\n');
    return;
  }
  if (command === 'publish') {
    const result = publishCandidate({
      runDir: requiredString(args.runDir, '--run-dir'),
      publicKeyFile: requiredString(args.publicKeyFile, '--public-key-file'),
      expectedSessionRestore: args.sessionRestore,
    });
    process.stdout.write(JSON.stringify(result, null, 2) + '\n');
    return;
  }
  if (command === 'archive-state') {
    process.stdout.write(JSON.stringify(archiveUpdaterState(args.runDir, requiredString(args.stage, '--stage')), null, 2) + '\n');
    return;
  }
  if (command === 'serve') {
    const { metadata } = loadRun(requiredString(args.runDir, '--run-dir'));
    const started = await createFeedServer({
      root: metadata.feed_root, port: args.port === undefined ? 0 : Number(args.port),
      fault: args.fault || 'none', delayMs: args.delayMs === undefined ? 5000 : Number(args.delayMs),
      logPath: metadata.server_log,
    });
    process.stdout.write(JSON.stringify({ endpoint: started.endpoint, fault: args.fault || 'none', pid: process.pid }) + '\n');
    const stop = () => { void started.close().finally(() => process.exit(0)); };
    process.once('SIGINT', stop);
    process.once('SIGTERM', stop);
    return new Promise(() => {});
  }
  if (command === 'launch') {
    process.stdout.write(JSON.stringify(launchVerificationApp({
      runRoot: args.runDir, appPath: args.app, platform: args.platform || process.platform,
      headless: args.headless === true,
    }), null, 2) + '\n');
    return;
  }
  if (command === 'status') {
    const { root, metadata } = loadRun(args.runDir);
    const { diagnostics } = resolveActiveProcess(root, metadata);
    const summary = summarizeDiagnostics(diagnostics);
    if (![metadata.previous_version, metadata.candidate_version].includes(summary.current_version)) {
      throw new Error('diagnostics current_version이 현재 회차의 이전·후보 버전과 다릅니다');
    }
    recordMarker(join(root, 'events.jsonl'), 'install-phase', {
      phase: summary.phase || 'unknown',
      version: summary.target_version || summary.current_version || undefined,
    });
    process.stdout.write(JSON.stringify(summary, null, 2) + '\n');
    return;
  }
  if (command === 'stop') {
    process.stdout.write(JSON.stringify(await stopOwnedApp(args.runDir, args.platform || process.platform)) + '\n');
    return;
  }
  throw new Error('알 수 없는 명령입니다: ' + command);
}

if (process.argv[1] && resolve(process.argv[1]) === resolve(new URL(import.meta.url).pathname)) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write('issue102-update-runner: ' + error.message + '\n');
    process.exitCode = 1;
  });
}
