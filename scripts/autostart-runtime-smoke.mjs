// Windows passive-start regression. Uses a worktree debug instance; no Run writes.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { devInstance, devEnvironment } from './dev-instance.mjs';
assert.equal(process.platform,'win32');
const root=fileURLToPath(new URL('..',import.meta.url));
const instance=devInstance(root),env=devEnvironment(instance);
const out=process.env.AUTOSTART_OUTPUT;assert.ok(out);
mkdirSync(out,{recursive:true});
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
function ps(code) {
  const result=spawnSync('powershell.exe',['-NoProfile','-Command',`[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); ${code}`],{encoding:'utf8',windowsHide:true,env:{...env,JAJAN_TEST_BINARY:instance.binary}});
  assert.equal(result.status,0,result.stderr);return JSON.parse(result.stdout.trim()||'null');
}
const native=()=>ps(`
Add-Type @'
using System; using System.Text; using System.Runtime.InteropServices; using System.Collections.Generic;
public class StartupWindows {
 public delegate bool Visit(IntPtr h,IntPtr l);
 [DllImport("user32.dll")] static extern bool EnumWindows(Visit f,IntPtr l);
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h,StringBuilder t,int n);
 [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
 public static object Scan() {
  var rows=new List<object>();
  EnumWindows((h,l)=>{uint p;GetWindowThreadProcessId(h,out p);var b=new StringBuilder(512);GetWindowText(h,b,512);if(b.Length>0)rows.Add(new {pid=p,title=b.ToString(),visible=IsWindowVisible(h)});return true;},IntPtr.Zero);
  uint fg;GetWindowThreadProcessId(GetForegroundWindow(),out fg);return new {foregroundPid=fg,windows=rows};
 }
}
'@
[StartupWindows]::Scan() | ConvertTo-Json -Depth 5 -Compress`);
function cli(args,ok=true) {
  const result=spawnSync(process.execPath,[join(root,'scripts/dev.mjs'),'cli',...args],{cwd:root,encoding:'utf8',timeout:20000,windowsHide:true,env});
  if(!ok && result.status!==0)return null;
  assert.equal(result.status,0,result.stdout+result.stderr);
  const value=JSON.parse(result.stdout);assert.equal(value.ok,true);return value;
}
const processes=()=>{const value=ps(`@(Get-CimInstance Win32_Process -Filter "name = 'jajan.exe'" | Where-Object { $_.ExecutablePath -eq $env:JAJAN_TEST_BINARY } | ForEach-Object { $_.ProcessId }) | ConvertTo-Json -Compress`);return value==null?[]:Array.isArray(value)?value:[value];};
assert.equal(processes().length,0,'This worktree app must not already be running');
const reg=()=>ps(`$k=Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name Jajan -ErrorAction SilentlyContinue; if($null -eq $k){'null'}else{@{Jajan=$k.Jajan}|ConvertTo-Json -Compress}`);
const runBefore=reg();
const records={root,branch:instance.name,instance:instance.id,binary:instance.binary,startMode:'--login-startup',runBefore};
const ids=[`autostart-visible-${Date.now()}`,`autostart-hidden-${Date.now()}`];
const journal=join(process.env.APPDATA,'app.jajan','notes',instance.id,'docs','.session-windows.json');
const hadJournal=existsSync(journal);
let child;
async function launch() {
  child=spawn(instance.binary,['--login-startup'],{cwd:root,env,stdio:'ignore',windowsHide:true});
  await new Promise((resolve,reject)=>{child.once('spawn',resolve);child.once('error',reject);});
  for(let i=0;i<60;i++){if(cli(['diagnostics'],false))return;await delay(200);}
  throw Error('Passive server did not become ready');
}
async function stop() {
  if(!child || child.exitCode!==null)return;
  const found=processes();assert.ok((Array.isArray(found)?found:[found]).includes(child.pid),'Owned PID executable identity changed');
  child.kill();await new Promise(resolve=>child.once('exit',resolve));
}
try {
  records.foregroundBefore=native().foregroundPid;
  await launch();records.firstPid=child.pid;
  const first=native();records.firstWindows=first.windows.filter(w=>w.pid===child.pid);records.foregroundAfter=first.foregroundPid;
  assert.equal(first.foregroundPid,records.foregroundBefore,'Login launch took foreground focus');
  if(!hadJournal) assert.equal(records.firstWindows.some(w=>w.visible),false,'Fresh login launch opened a visible window');
  assert.equal(records.firstWindows.some(w=>w.title.includes('메모 관리') && w.visible),false);
  const diagnostics=cli(['diagnostics']);assert.equal(diagnostics.instance,instance.id);
  assert.equal(diagnostics.autostart.available,false);assert.equal(diagnostics.autostart.os_status,'unavailable');
  records.autostart=diagnostics.autostart;
  for(const [i,id] of ids.entries()) {
    cli(['open','--id',id,'--markdown',`로그인 복원 검증 ${i} · ${instance.id}`]);
  }
  cli(['close','--id',ids[1]]);
  const before=ids.map(id=>cli(['inspect','--id',id]));
  for(const item of before)assert.equal(item.instance,instance.id);
  for(let i=0;i<60;i++) {
    if(existsSync(journal)){
      const saved=JSON.parse(readFileSync(journal,'utf8'));
      if(ids.every(id=>saved.windows.some(w=>w.window.label===`note-${id}` && w.window.visible===(id===ids[0]))))break;
    }
    if(i===59)throw Error('Window journal did not reach expected visible/hidden state');
    await delay(200);
  }
  records.inspectBefore=before;records.journal=journal;
  await stop();
  const foreground=native().foregroundPid;
  await launch();records.secondPid=child.pid;
  const after=ids.map(id=>cli(['inspect','--id',id]));
  records.inspectAfter=after;
  assert.equal(after[0].visible,true);assert.equal(after[1].visible,false);
  assert.equal(after[0].note.markdown,before[0].note.markdown);
  for(const [i,item] of after.entries()) {
    assert.equal(item.instance,instance.id);
    assert.deepEqual(item.position,before[i].position);
    assert.deepEqual(item.size,before[i].size);
    assert.equal(item.always_on_top,before[i].always_on_top);
  }
  const restored=native();records.restoredWindows=restored.windows.filter(w=>w.pid===child.pid);
  assert.equal(restored.foregroundPid,foreground,'Restoration took foreground focus');
  assert.ok(records.restoredWindows.some(w=>w.title.includes(ids[0]) && w.visible));
  assert.equal(records.restoredWindows.some(w=>w.title.includes('메모 관리') && w.visible),false);
  records.runAfter=reg();assert.deepEqual(records.runAfter,runBefore,'Startup Run entry changed during debug validation');
  records.ok=true;
} finally {
  if(child && child.exitCode===null){for(const id of ids)cli(['close','--id',id],false);await delay(500);}
  await stop();
  writeFileSync(join(out,'runtime.json'),JSON.stringify(records,null,2)+'\n');
}
console.log(JSON.stringify({ok:true,instance:instance.id,firstPid:records.firstPid,secondPid:records.secondPid,output:join(out,'runtime.json')}));
