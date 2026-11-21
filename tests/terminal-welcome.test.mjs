import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
// Approved wireframe replaces immediate app launch/details with selection + Run
// and a persistent selected-command preview. Removed modes cannot auto launch.
const flush=async()=>{for(let i=0;i<20;i++)await Promise.resolve();};
const folder=path=>({path,name:path.split('/').at(-1),parent:path.slice(0,path.lastIndexOf('/')),command:`cd -- '${path}'`});
const shell={id:'shell',label:'셸',command:"'/bin/zsh' -i"};
function ui({exitCode=null,grid=null,ipc=null,restoreSupported=true,plan=null,restored=false,autoRestore=false,legacyDetached=false,kind='terminal',path=null,fail=false,focused=true,picker=null,folders=[],choices=[{id:'git',label:'git log',command:"'/usr/bin/git' log --oneline --decorate -20"},shell]}={}){
 const elements={},calls=[],intervals=[];
 const element=id=>elements[id]??=({id,hidden:false,textContent:'',value:'',children:[],attrs:{},dataset:{},focus(){this.focused=true;},addEventListener(){},setAttribute(k,v){this.attrs[k]=v;},getAttribute(k){return this.attrs[k];},replaceChildren(){this.children=[];},append(...children){this.children.push(...children);}});
 element('rename-title');
 const info={exit_code:exitCode,restore_supported:restoreSupported,restore_plan:plan,restored,auto_restore:autoRestore,...(legacyDetached?{detached_session:true}:{}),kind,id:'terminal-123',title:'터미널',last_path:path};
 const context={innerWidth:720,innerHeight:460,navigator:{platform:'MacIntel'},setTimeout,clearTimeout,setInterval:fn=>intervals.push(fn),ResizeObserver:class{observe(){}},Terminal:class{constructor(){this.cols=80;this.rows=24;this.options={fontSize:13};}loadAddon(){}open(){}onData(){}attachCustomKeyEventHandler(){}focus(){}write(){}},FitAddon:{FitAddon:class{fit(){}}},document:{hasFocus:()=>focused,getElementById:element,createElement:()=>element('element'+Math.random()),querySelector:()=>element('header'),querySelectorAll:()=>[]},window:{installTerminalIme(){},__TAURI__:{core:{Channel:class{},invoke:async(name,args)=>{
  calls.push([name,args]);
  if(ipc && (name==='terminal_resize'||name==='terminal_start'))return ipc(name,args);
  if(name==='terminal_folder'&&args?.path===null&&picker)return await picker();
  if(name==='terminal_folder')info.last_path=args?.path||'/tmp/chosen';
  if(name==='terminal_rename')info.title=args.title;
  if(name==='terminal_welcome')return {info,choices,folders,home:'/home/test',location:folder(info.last_path||'/home/test')};
  if(name==='terminal_start'&&fail)throw Error('missing');
  return {...info};
 }},window:{getCurrentWindow:()=>({startDragging:async()=>{},startResizeDragging:async()=>{}})}}}};
 context.document.addEventListener=(name,listener)=>{if(grid && name==='visibilitychange')grid.visibilityChanged=listener;};
 if(grid){
   const host=element('terminal'),screen={};
   let terminal;
   host.querySelector=()=>screen;
   Object.defineProperties(host,{
     clientWidth:{get:()=>host.hidden?0:grid.width},
     clientHeight:{get:()=>host.hidden?0:grid.height},
   });
   context.Terminal=class extends context.Terminal{
     constructor(){super();terminal=this;}
     resize(cols,rows){Object.assign(this,{cols,rows});}
   };
   context.getComputedStyle=node=>node===screen
     ? {width:terminal.cols*(terminal.options.fontSize-3),height:terminal.rows*20}
     : {paddingLeft:0,paddingRight:0,paddingTop:0,paddingBottom:0};
   context.matchMedia=()=>({matches:false});
   context.ResizeObserver=class{constructor(callback){grid.observed=callback;}observe(){}};
 }
 vm.runInNewContext(readFileSync(new URL('../ui/content-size.js',import.meta.url),'utf8'),context);
 vm.runInNewContext(readFileSync(new URL('../ui/terminal.js',import.meta.url),'utf8'),context);return {elements,calls,intervals,info};
}
test('no history uses home preview and retains folder picker and shell without autostart',async()=>{const {elements,calls}=ui();await flush();assert.equal(elements['home-hint'].hidden,false);assert.equal(elements['recent-folders'].children.length,0);assert.equal(calls.some(([n])=>n==='terminal_start'),false);assert.equal(typeof elements['choose-folder'].onclick,'function');assert.ok(elements.recommendations.children.some(b=>b.dataset.choice==='shell'));assert.match(elements['command-preview'].textContent,/cd -- '\/home\/test'/);});
test('recent folders show three, expand, select locally and update preview',async()=>{const folders=['/a/one','/b/two','/c/three','/d/four'].map(folder);const {elements,calls}=ui({path:folders[0].path,folders});await flush();assert.equal(elements['recent-folders'].children.length,3);assert.equal(elements['more-folders'].hidden,false);await elements['more-folders'].onclick();assert.equal(elements['recent-folders'].children.length,4);const row=elements['recent-folders'].children[3];row.onclick();assert.equal(row.attrs['aria-pressed'],'true');assert.equal(elements['recent-folders'].children[0].attrs['aria-pressed'],'false');assert.ok(elements['command-preview'].textContent.includes(folders[3].command));assert.equal(calls.some(([n])=>n==='terminal_folder'||n==='terminal_start'),false);await elements['run-task'].onclick();const writes=calls.filter(([n])=>n==='terminal_folder'||n==='terminal_start');assert.equal(writes[0][0],'terminal_folder');assert.equal(writes[0][1].path,folders[3].path);assert.equal(writes[1][0],'terminal_start');});
test('task selection updates pressed state and exact preview; Run starts only once',async()=>{const {elements,calls}=ui();await flush();elements.recommendations.children[1].onclick();assert.equal(elements.recommendations.children[1].attrs['aria-pressed'],'true');assert.equal(elements.recommendations.children[0].attrs['aria-pressed'],'false');assert.ok(elements['command-preview'].textContent.endsWith(shell.command));assert.equal(calls.some(([n])=>n==='terminal_start'),false);await elements['run-task'].onclick();await elements['run-task'].onclick();assert.equal(calls.filter(([n])=>n==='terminal_start').length,1);assert.equal(calls.find(([n])=>n==='terminal_start')[1].choice,'shell');assert.equal(elements.welcome.hidden,true);await elements.close.onclick();assert.equal(calls.at(-1)[0],'terminal_close');});
test('shell remains the only task when no recommended app is installed',async()=>{const {elements,calls}=ui({choices:[shell]});await flush();assert.equal(elements.recommendations.children.length,1);assert.equal(elements.recommendations.children[0].dataset.choice,'shell');await elements['run-task'].onclick();assert.equal(calls.find(([n])=>n==='terminal_start')[1].choice,'shell');});
test('picker refreshes location and preview without starting',async()=>{const {elements,calls}=ui();await flush();await elements['choose-folder'].onclick();assert.equal(elements['home-hint'].hidden,true);assert.ok(elements['command-preview'].textContent.includes('/tmp/chosen'));assert.equal(calls.some(([n])=>n==='terminal_start'),false);});
test('failed Run preserves Welcome, restores Run focus and permits retry',async()=>{const {elements,calls}=ui({fail:true});await flush();await elements['run-task'].onclick();assert.equal(elements.welcome.hidden,false);assert.equal(elements.terminal.hidden,true);assert.equal(elements['run-task'].focused,true);assert.equal(elements.welcome.attrs['aria-busy'],'false');assert.match(elements.status.textContent,/missing/);await elements['run-task'].onclick();assert.equal(calls.filter(([n])=>n==='terminal_start').length,2);});
test('failed launch does not move focus when window is inactive',async()=>{const {elements}=ui({fail:true,focused:false});await flush();await elements['run-task'].onclick();assert.notEqual(elements['run-task'].focused,true);});
test('removed agent mode cannot auto launch a PTY',async()=>{const {elements,calls}=ui({kind:'agent'});await flush();assert.equal(calls.some(([n])=>n==='terminal_start'||n==='terminal_welcome'),false);assert.match(elements.status.textContent,/지원하지 않습니다/);});
test('Welcome removes rename while polling still reads metadata',async()=>{assert.equal(readFileSync(new URL('../ui/terminal.html',import.meta.url),'utf8').includes('id="rename-form"'),false);const {elements,calls,intervals}=ui();await flush();await elements['run-task'].onclick();await intervals[0]();await flush();assert.equal(calls.at(-1)[0],'terminal_info');});
test('long folder path and exact command remain available',async()=>{const path='/project/'+('긴 경로/'.repeat(80));const {elements}=ui({path});await flush();assert.equal(elements['recent-folders'].children[0].title,path);assert.ok(elements['command-preview'].textContent.includes(folder(path).command));});

test('cancelled delayed picker preserves local folder selection and blocks Run while pending',async()=>{let cancel;const picker=()=>new Promise(resolve=>{cancel=()=>resolve({folder_selected:false});});const folders=['/a/one','/b/two'].map(folder);const {elements,calls}=ui({path:folders[0].path,folders,picker});await flush();elements['recent-folders'].children[1].onclick();const preview=elements['command-preview'].textContent;const pending=elements['choose-folder'].onclick();await flush();assert.equal(elements['run-task'].disabled,true);await elements['run-task'].onclick();assert.equal(calls.some(([n])=>n==='terminal_start'),false);cancel();await pending;assert.equal(elements['run-task'].disabled,false);assert.equal(elements['recent-folders'].children[1].attrs['aria-pressed'],'true');assert.equal(elements['command-preview'].textContent,preview);assert.equal(calls.filter(([n])=>n==='terminal_welcome').length,1);});

test('selecting recent folder from home clears the home location hint',async()=>{const {elements}=ui({folders:[folder('/project/recent')]});await flush();assert.equal(elements['home-hint'].hidden,false);elements['recent-folders'].children[0].onclick();assert.equal(elements['home-hint'].hidden,true);assert.match(elements['command-preview'].textContent,/project\/recent/);});

test('approved wireframe selects a task without starting until Run',async()=>{const {elements,calls}=ui();await flush();await elements.recommendations.children[0].onclick();assert.equal(calls.filter(([n])=>n==='terminal_start').length,0);assert.equal(typeof elements['run-task'].onclick,'function');await elements['run-task'].onclick();assert.equal(calls.filter(([n])=>n==='terminal_start').length,1);});

test('restoration uses edited plan only after explicit restore',async()=>{
  const {elements,calls}=ui({plan:'df -h'});await flush();
  assert.equal(elements['restore-plan'].value,'df -h');
  assert.equal(elements['welcome-panel'].hidden,true);
  assert.equal(calls.some(([name])=>name==='terminal_start'),false);
  elements['restore-plan'].value='pwd';
  await elements['restore-run'].onclick();
  assert.equal(calls.find(([name])=>name==='terminal_start')[1].plan,'pwd');
});
test('cancel opens welcome without starting a shell',async()=>{
  const {elements,calls}=ui({plan:'codex'});await flush();
  elements['restore-cancel'].onclick();
  assert.equal(elements['welcome-panel'].hidden,false);
  assert.equal(elements['restore-panel'].hidden,true);
  assert.equal(calls.some(([name])=>name==='terminal_start'),false);
});

test('only a verified monitor in a restored window starts automatically',async()=>{
 for(const [restored,autoRestore,expected] of [[true,true,1],[true,false,0],[false,true,0]]){
  const {calls}=ui({plan:'htop',restored,autoRestore});
  await flush();
  const starts=calls.filter(([name])=>name==='terminal_start');
  assert.equal(starts.length,expected);
  if(expected) assert.equal(starts[0][1].choice,'auto-monitor');
 }
});


test('unsupported restoration shows an explanation and blocks auto and edited execution',async()=>{
 const {elements,calls}=ui({plan:'htop',restored:true,autoRestore:true,restoreSupported:false});
 await flush();
 assert.match(elements.status.textContent,/아직 지원하지 않습니다/);
 elements['restore-plan'].value='pwd';
 await elements['restore-run'].onclick();
 assert.equal(calls.some(([name])=>name==='terminal_start'),false);
 elements['restore-cancel'].onclick();
 assert.equal(elements['welcome-panel'].hidden,false);
 await elements['run-task'].onclick();
 assert.equal(calls.filter(([name])=>name==='terminal_start').length,1);
});


test('a recreated webview for a retained exited session cannot rerun the saved plan',async()=>{
 const {elements,calls}=ui({exitCode:0,plan:'htop',restored:true,autoRestore:true});
 await flush();
 assert.equal(elements.welcome.hidden,true);
 assert.equal(elements.terminal.hidden,false);
 assert.equal(calls.some(([name])=>name==='terminal_welcome'||name==='terminal_start'),false);
 await elements.close.onclick();
 assert.equal(calls.at(-1)[0],'terminal_close');
});

function gridFixture(t, options={}){
  t.mock.timers.enable({apis:['setTimeout']});
  const grid={width:800,height:480};
  const result=ui({...options,grid});
  function viewport(width,height=480){
    Object.assign(grid,{width,height});
    grid.observed();
    t.mock.timers.tick(60);
  }
  const resizes=()=>result.calls.filter(([name])=>name==='terminal_resize')
    .map(([,args])=>[args.cols,args.rows]);
  return {...result,grid,viewport,resizes};
}

test('a PTY starts with its measured grid and equivalent later measurements send no resize',async t=>{
  const {elements,calls,viewport,resizes}=gridFixture(t);
  await flush();
  await elements['run-task'].onclick();
  const start=calls.find(([name])=>name==='terminal_start')[1];
  assert.deepEqual([start.cols,start.rows],[80,24]);
  for(let i=0;i<100;i++){viewport(800+i%10);await flush();}
  assert.equal(resizes().length,0);
  viewport(810);
  await flush();
  assert.deepEqual(resizes(),[[81,24]]);
});

test('resizing during start and a delayed resize converges to the latest measured grid',async t=>{
  const requests=[];
  const {elements,viewport,resizes}=gridFixture(t,{ipc:()=>new Promise(resolve=>requests.push(resolve))});
  await flush();
  const start=elements['run-task'].onclick();
  await flush();
  viewport(900);
  assert.deepEqual(resizes(),[]);
  requests.shift()();
  await start;
  assert.deepEqual(resizes(),[[90,24]]);
  for(const width of [910,920,895,800])viewport(width);
  assert.deepEqual(resizes(),[[90,24]]);
  requests.shift()();
  await flush();
  assert.deepEqual(resizes(),[[90,24],[80,24]]);
  requests.shift()();
  await flush();
  viewport(800);
  assert.equal(resizes().length,2);
});

test('a failed resize retries once, then a fresh measurement may retry the same grid',async t=>{
  let attempts=0;
  const {elements,viewport,resizes}=gridFixture(t,{ipc:async name=>{
    if(name==='terminal_resize' && ++attempts<=2)throw Error('resize unavailable');
  }});
  await flush();
  await elements['run-task'].onclick();
  viewport(900);
  await flush();
  t.mock.timers.tick(1000);
  await flush();
  t.mock.timers.tick(1000);
  await flush();
  assert.deepEqual(resizes(),[[90,24],[90,24]]);
  assert.match(elements.status.textContent,/resize unavailable/);
  viewport(901);
  await flush();
  viewport(903);
  assert.deepEqual(resizes(),[[90,24],[90,24],[90,24]]);
});

test('restored PTY, hidden host and font change use the visible grid',async t=>{
  const {elements,calls,grid,viewport,resizes}=gridFixture(t,{restored:true,plan:'htop',autoRestore:true});
  await flush();
  assert.equal(calls.find(([name])=>name==='terminal_start')[1].cols,80);
  elements.terminal.hidden=true;
  viewport(990);
  assert.deepEqual(resizes(),[]);
  elements.terminal.hidden=false;
  grid.visibilityChanged();
  t.mock.timers.tick(60);
  await flush();
  assert.deepEqual(resizes(),[[99,24]]);
  elements['font-size'].onclick();
  t.mock.timers.tick(60);
  await flush();
  assert.deepEqual(resizes(),[[99,24],[90,24]]);
});

test('a failed launch may restart with a different grid, and a failed stale resize yields to newer cells',async t=>{
  let launch=0, rejectResize;
  const {elements,calls,viewport,resizes}=gridFixture(t,{ipc:name=>{
    if(name==='terminal_start'){
      if(++launch===1)throw Error('launch failed');
    }else if(!rejectResize){
      return new Promise((_,reject)=>{rejectResize=reject;});
    }
  }});
  await flush();
  await elements['run-task'].onclick();
  viewport(900);
  await elements['run-task'].onclick();
  assert.deepEqual(calls.filter(([name])=>name==='terminal_start').map(([,args])=>args.cols),[80,90]);
  assert.deepEqual(resizes(),[]);
  viewport(910);
  viewport(930);
  rejectResize(Error('resize unavailable'));
  await flush();
  assert.deepEqual(resizes(),[[91,24],[93,24]]);
  t.mock.timers.tick(1000);
  assert.equal(resizes().length,2);
});

test('an exit observed during a pending resize blocks the queued latest grid',async t=>{
  let finishResize;
  const {elements,calls,intervals,info,viewport,resizes}=gridFixture(t,{ipc:name=>{
    if(name==='terminal_resize')return new Promise(resolve=>{finishResize=resolve;});
  }});
  await flush();
  await elements['run-task'].onclick();
  viewport(900);
  assert.deepEqual(resizes(),[[90,24]]);
  viewport(920);
  assert.deepEqual(resizes(),[[90,24]]);

  // The periodic terminal_info response records natural exit while resize is
  // still in flight. Completing that resize must not launch the pending grid.
  info.exit_code=0;
  const infoCall=intervals[0]();
  await flush();
  finishResize();
  await infoCall;
  await flush();
  assert.deepEqual(resizes(),[[90,24]]);
  assert.equal(calls.filter(([name])=>name==='terminal_start').length,1);
});

test('legacy detached flag cannot automatically reattach or execute a shell',async()=>{
 const {elements,calls}=ui({legacyDetached:true});await flush();
 assert.equal(calls.some(([name])=>name==='terminal_start'),false);
 assert.equal(elements.welcome.hidden,false);
 elements.recommendations.children.find(b=>b.dataset.choice==='shell').onclick();
 await elements['run-task'].onclick();
 assert.equal(calls.filter(([name])=>name==='terminal_start').length,1);
 assert.equal(calls.find(([name])=>name==='terminal_start')[1].choice,'shell');
});
