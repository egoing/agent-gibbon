// Build two genuine, schema-identical NSIS versions in a private temporary tree.
// This prepares artifacts and a loopback feed; it never installs or publishes them.
import { cp, mkdir, mkdtemp, readFile, writeFile, symlink, readdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn, execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { generateKeyPairSync, randomBytes, sign, createHash } from 'node:crypto';
import { devInstance } from './dev-instance.mjs';
import { checkVersion, setVersion } from './release-version.mjs';

if (process.platform !== 'win32') throw Error('Windows NSIS verification only');
const root = fileURLToPath(new URL('..', import.meta.url));
if (checkVersion(undefined, root) !== '0.3.12') throw Error('This approved isolated pair requires source version 0.3.12; review a new compatible pair before changing it.');
if (!(await readFile(join(root,'src-tauri/src/instance.rs'),'utf8')).includes('JAJAN_VERIFICATION_INSTANCE')) {
  throw Error('Integrate the PTY verification namespace support before preparing the combined trial.');
}
const workspace = await mkdtemp(join(tmpdir(), 'jajan-roundtrip-5170-'));
// Both versions share this trial's namespace; other concurrent verification apps do not.
const verificationInstance = `dev-${createHash('sha256').update(workspace).digest('hex').slice(0,24)}`;
const source = join(workspace, 'source');
await mkdir(source);
for (const name of ['src-tauri', 'ui', 'scripts', 'docs', 'bundled-skills', '.agents', 'package.json', 'pnpm-lock.yaml']) {
  await cp(join(root, name), join(source, name), {recursive:true, filter:path => !['target', 'node_modules', '.tmp'].includes(basename(path))});
}
await symlink(join(root, 'node_modules'), join(source, 'node_modules'), 'junction');
const responses = new Map();
const server = createServer((req, res) => {
  const body = responses.get(req.url);
  if (!body) { res.writeHead(503); res.end('Verification feed not ready'); return; }
  res.writeHead(200, {'Content-Length':body.length}); res.end(body);
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}/`;
const {privateKey, publicKey} = generateKeyPairSync('ed25519');
const keyId = randomBytes(8);
const pubPacket = Buffer.concat([Buffer.from('Ed'),keyId,publicKey.export({format:'der',type:'spki'}).subarray(-32)]);
const publicKeyText = Buffer.from(`untrusted comment: isolated verification key\n${pubPacket.toString('base64')}\n`).toString('base64');
const config = {identifier:'app.jajan.roundtrip5170', productName:'Jajan Roundtrip Verification 5170', mainBinaryName:'jajan-roundtrip-5170',
  build:{beforeBuildCommand:''}, bundle:{targets:['nsis'],createUpdaterArtifacts:false,windows:{nsis:{installMode:'currentUser'},allowDowngrades:true}},
  plugins:{updater:{pubkey:publicKeyText,requireSignedVersion:true,dangerousInsecureTransportProtocol:true,windows:{installMode:'quiet',installerArgs:['/NS']}}}};
const configPath = join(workspace, 'verification.json');
await writeFile(configPath, JSON.stringify(config));
const target = join(workspace, 'target');
const env = {...process.env, CARGO_TARGET_DIR:target, CARGO_BUILD_BUILD_DIR:devInstance(root).buildDir,
  TAURI_UPDATER_PUBLIC_KEY:publicKeyText, JAJAN_VERIFICATION_UPDATE_URL:`${base}latest.json`, JAJAN_VERIFICATION_STANDARD_TIMING:'1',
  JAJAN_VERIFICATION_INSTANCE:verificationInstance};
async function run(args) {
  await new Promise((resolve,reject) => {
    const child=spawn(process.execPath,args,{cwd:source,env,stdio:'inherit'});
    child.on('error',reject); child.on('exit',code=>code===0?resolve():reject(Error(`Build exited ${code}`)));
  });
}
const plan = {workspace,source,target,base,verificationInstance,identifier:config.identifier,productName:config.productName,
  binaryName:'jajan-roundtrip-5170.exe',installDir:join(workspace,'installed'),previous:'0.3.12',latest:'0.3.13',artifacts:{}};
await writeFile(join(workspace,'plan.json'),JSON.stringify(plan,null,2));
console.log(`ROUNDTRIP_WORKSPACE=${workspace}`);
for (const version of [plan.previous,plan.latest]) {
  if (version !== plan.previous) setVersion(version,source);
  await run([join(source,'scripts/build-editor.mjs')]);
  await run([join(source,'node_modules/@tauri-apps/cli/tauri.js'),'build','--ci','--bundles','nsis','--features','verification-roundtrip','--config',configPath]);
  const identity = JSON.parse(execFileSync(join(target,'release',plan.binaryName),['--instance-info'],{encoding:'utf8',env}));
  if (identity.ok !== true || identity.instance !== verificationInstance) throw Error('Built verification instance does not match this trial.');
  const installers = (await readdir(join(target,'release/bundle/nsis'))).filter(name=>name.endsWith('.exe') && name.includes(`_${version}_`));
  if (installers.length!==1) throw Error(`Expected one installer for ${version}`);
  const bytes=await readFile(join(target,'release/bundle/nsis',installers[0]));
  const filename=`Jajan_${version}_x64-setup.exe`;
  const signature=sign(null,createHash('blake2b512').update(bytes).digest(),privateKey);
  const trusted=`timestamp:${Math.floor(Date.now()/1000)}\tfile:${filename}\tversion:${version}`;
  const globalSignature=sign(null,Buffer.concat([signature,Buffer.from(trusted)]),privateKey);
  const encoded=Buffer.from(`untrusted comment: isolated verification signature\n${Buffer.concat([Buffer.from('ED'),keyId,signature]).toString('base64')}\ntrusted comment: ${trusted}\n${globalSignature.toString('base64')}\n`).toString('base64');
  const output=join(workspace,filename); await writeFile(output,bytes);
  await writeFile(`${output}.sig`,encoded);
  responses.set(`/v${version}/${filename}`,bytes);
  const manifest={version,platforms:{'windows-x86_64':{url:`${base}v${version}/${filename}`,signature:encoded}}};
  responses.set(version===plan.previous?'/previous.json':'/latest.json',Buffer.from(JSON.stringify(manifest)));
  plan.artifacts[version]={installer:output,sha256:createHash('sha256').update(bytes).digest('hex'),instance:identity.instance};
}
await writeFile(join(workspace,'public.key'),publicKeyText);
await writeFile(join(workspace,'plan.json'),JSON.stringify(plan,null,2));
console.log(`ROUNDTRIP_READY=${join(workspace,'plan.json')}`);
console.log('Loopback feed is running. Stop this process after the verification. No installation was executed.');
