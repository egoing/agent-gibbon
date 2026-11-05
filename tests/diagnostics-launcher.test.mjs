import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const script=fileURLToPath(new URL('../scripts/diagnostics.mjs',import.meta.url));
test('ordinary contributor build does not fetch official DSN or silently enable transmission',()=>{
 const env={...process.env};delete env.AGENT_GIBBON_SENTRY_DSN;
 const p=spawnSync(process.execPath,[script,'build'],{env,encoding:'utf8',timeout:3000});assert.equal(p.status,1);assert.match(p.stderr,/requires/);
});
test('invalid DSN is rejected without printing the supplied value',()=>{
 const value='https://PRIVATE_TOKEN:PRIVATE_PASSWORD@example.invalid/42';
 const p=spawnSync(process.execPath,[script,'build'],{env:{...process.env,AGENT_GIBBON_SENTRY_DSN:value},encoding:'utf8',timeout:3000});assert.equal(p.status,1);assert(!p.stdout.includes('PRIVATE'));assert(!p.stderr.includes('PRIVATE'));
});
