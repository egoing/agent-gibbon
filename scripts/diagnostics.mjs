#!/usr/bin/env node
// Explicit diagnostic builds: obtain the public ingest DSN in memory, never print it.
import {spawn, spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const [mode,...args]=process.argv.slice(2);
const modes={build:['build'],open:['cli','open',...args],status:['cli','diagnostics'],probe:['cli','diagnostics-probe']};
if(!modes[mode]){console.error('Usage: diagnostics.mjs build|open|status|probe');process.exit(1);}
const env={...process.env,AGENT_GIBBON_DIAGNOSTICS:'1'};
if(mode==='build'){
 let dsn=env.AGENT_GIBBON_SENTRY_DSN?.trim();
 if(!dsn && args.includes('--official')){
  const api=path=>spawnSync('gh',['api',path],{encoding:'utf8',windowsHide:true,timeout:15000});
  // Forks/contributors must not accidentally send their diagnostics to the official project.
  const permission=api('repos/egoing/agent-gibbon');let writer=false;
  try{writer=permission.status===0&&JSON.parse(permission.stdout).permissions?.push===true;}catch{}
  if(writer){const result=api('repos/egoing/agent-gibbon/actions/variables/AGENT_GIBBON_SENTRY_DSN');try{if(result.status===0)dsn=JSON.parse(result.stdout).value;}catch{}}
 }
 let valid=false;try{const url=new URL(dsn);valid=url.protocol==='https:'&&Boolean(url.username)&&!url.password&&/^\/\d+$/.test(url.pathname);}catch{}
 if(!valid){console.error('Diagnostic build requires AGENT_GIBBON_SENTRY_DSN or --official with repository write access. DSN value withheld.');process.exit(1);}
 env.AGENT_GIBBON_SENTRY_DSN=dsn;
}
// Build/launch processes need the ingest DSN, never account management credentials.
for(const key of ['SENTRY_AUTH_TOKEN','GH_TOKEN','GITHUB_TOKEN'])delete env[key];
const child=spawn(process.execPath,[fileURLToPath(new URL('./dev.mjs',import.meta.url)),...modes[mode]],{env,stdio:'inherit',windowsHide:true});
for(const signal of ['SIGINT','SIGTERM'])process.on(signal,()=>child.kill(signal));
child.on('error',()=>{console.error('Diagnostic launcher failed.');process.exitCode=1;});
child.on('exit',code=>{process.exitCode=code??1;});
