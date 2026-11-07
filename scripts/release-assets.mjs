import { createHash, createPublicKey, verify } from 'node:crypto';
import { readdirSync, readFileSync, statSync, writeFileSync, renameSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const platforms = [
  {
    target: 'darwin-aarch64',
    archive: version => `Agent Gibbon_${version}_darwin-aarch64.app.tar.gz`,
    signature: version => `Agent Gibbon_${version}_darwin-aarch64.app.tar.gz.sig`,
  },
  {
    target: 'darwin-x86_64',
    archive: version => `Agent Gibbon_${version}_darwin-x86_64.app.tar.gz`,
    signature: version => `Agent Gibbon_${version}_darwin-x86_64.app.tar.gz.sig`,
  },
  {
    target: 'windows-x86_64',
    archive: version => `Agent Gibbon_${version}_x64-setup.exe`,
    signature: version => `Agent Gibbon_${version}_x64-setup.exe.sig`,
  },
];

function installerNames(version) {
  return [
    `Agent Gibbon_${version}_aarch64.dmg`,
    `Agent Gibbon_${version}_x64.dmg`,
    `Agent Gibbon_${version}_x64-setup.exe`,
  ];
}

// Minisign's Ed25519 prehash format, also used by Tauri's Rust verifier.
// Validate both the payload signature and its authenticated trusted comment.
function verifySignature(encodedSignature, encodedPublicKey, payload, version, filename) {
  const decode = value => {
    const encoded = value.trim();
    const bytes = Buffer.from(encoded, 'base64');
    if (bytes.toString('base64') !== encoded) throw new Error('Invalid signature Base64');
    return bytes;
  };
  const lines = decode(encodedSignature).toString('utf8').trim().split(/\r?\n/);
  const keyLines = decode(encodedPublicKey).toString('utf8').trim().split(/\r?\n/);
  if (lines.length !== 4 || !lines[2].startsWith('trusted comment: ') || keyLines.length !== 2) throw new Error('Invalid signature structure');
  const packet = decode(lines[1]), publicPacket = decode(keyLines[1]), globalSignature = decode(lines[3]);
  if (packet.length !== 74 || publicPacket.length !== 42 || globalSignature.length !== 64 || packet.subarray(0, 2).toString() !== 'ED' || publicPacket.subarray(0, 2).toString() !== 'Ed' || !packet.subarray(2,10).equals(publicPacket.subarray(2,10))) throw new Error('Invalid signature key or algorithm');
  const publicKey = createPublicKey({ key: Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),publicPacket.subarray(10)]),format:'der',type:'spki' });
  const trusted = lines[2].slice('trusted comment: '.length);
  if (!verify(null, createHash('blake2b512').update(payload).digest(), publicKey, packet.subarray(10)) || !verify(null, Buffer.concat([packet.subarray(10),Buffer.from(trusted)]), publicKey, globalSignature)) throw new Error('Invalid cryptographic signature');
  const fields = trusted.split('\t');
  if (fields.filter(f => f.startsWith('version:')).length !== 1 || !fields.includes(`version:${version}`) || fields.filter(f => f.startsWith('file:')).length !== 1 || !fields.includes(`file:${filename}`)) throw new Error('Invalid signed version or platform filename signature');
}

function validateFile(directory, name) {
  const path = resolve(directory, name);
  if (!statSync(path).isFile() || statSync(path).size === 0) throw new Error(`Empty or invalid release asset: ${name}`);
  return path;
}

function atomicWrite(path, contents) {
  const temporaryPath = `${path}.tmp`;
  writeFileSync(temporaryPath, contents, { flag: 'w', mode: 0o600 });
  renameSync(temporaryPath, path);
}

export function prepareAssets(directory, version, options = {}) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) throw new Error('Invalid release version');
  const installers = installerNames(version);
  const found = readdirSync(directory).sort();

  if (!options.updater) {
    const expected = installers.toSorted();
    if (JSON.stringify(found) !== JSON.stringify(expected)) throw new Error('Expected exactly two macOS DMGs and one Windows installer for this version');
    const lines = expected.map(name => `${createHash('sha256').update(readFileSync(validateFile(directory, name))).digest('hex')}  ${name}`);
    atomicWrite(resolve(directory, 'SHA256SUMS.txt'), `${lines.join('\n')}\n`);
    return;
  }

  const repository = options.repository ?? process.env.GITHUB_REPOSITORY ?? 'egoing/agent-gibbon';
  if (typeof repository !== 'string' || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) {
    throw new Error('A valid GitHub repository (owner/name) is required to prepare updater assets');
  }

  const updaterAssets = platforms.flatMap(platform => [platform.archive(version), platform.signature(version), `${platform.target}.pub`]);
  const expected = [...new Set([...installers, ...updaterAssets])].sort();
  const existing = found.filter(name => name !== 'latest.json' && name !== 'SHA256SUMS.txt');
  if (JSON.stringify(existing) !== JSON.stringify(expected)) {
    throw new Error('Expected exactly three installers and signed updater assets for darwin-aarch64, darwin-x86_64, and windows-x86_64');
  }

  const signatures = new Map();
  for (const platform of platforms) {
    const payload = readFileSync(validateFile(directory, platform.archive(version)));
    const publicKey = readFileSync(validateFile(directory, `${platform.target}.pub`), "utf8").trim();
    const expectedKey = options.expectedPublicKey ?? (process.env.RELEASE_TAG ? process.env.TAURI_UPDATER_PUBLIC_KEY : undefined);
    if ((process.env.RELEASE_TAG && !expectedKey) || (expectedKey && publicKey !== expectedKey.trim())) throw new Error("Updater signature public key does not match release trust root");
    const signature = readFileSync(validateFile(directory, platform.signature(version)), 'utf8').trim();
    verifySignature(signature, publicKey, payload, version, platform.archive(version));
    signatures.set(platform.target, signature);
  }

  const releaseBase = `https://github.com/${repository}/releases/download/v${version}/`;
  const manifest = {
    version,
    notes: '',
    platforms: Object.fromEntries(platforms.map(platform => {
      const filename = platform.archive(version);
      return [platform.target, {
        url: `${releaseBase}${encodeURIComponent(filename)}`,
        signature: signatures.get(platform.target),
      }];
    })),
  };
  const checksums = expected.map(name => `${createHash('sha256').update(readFileSync(validateFile(directory, name))).digest('hex')}  ${name}`);

  // Complete all validation before replacing an existing manifest or writing checksums.
  atomicWrite(resolve(directory, 'latest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
  atomicWrite(resolve(directory, 'SHA256SUMS.txt'), `${checksums.join('\n')}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const [directory, version, mode] = process.argv.slice(2);
    prepareAssets(directory, version, mode === 'updater' ? { updater: true } : undefined);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
