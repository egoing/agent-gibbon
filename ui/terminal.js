'use strict';
const { Channel } = window.__TAURI__.core;
const invoke = window.gibbonInvoke || window.__TAURI__.core.invoke;
const win = window.__TAURI__.window.getCurrentWindow();
const status = document.getElementById('status');
const clock = () => window.performance?.now?.() || 0;
const terminal = new Terminal({fontSize:13,fontFamily:'Menlo, Monaco, monospace',cursorBlink:true,scrollback:3000,allowProposedApi:false,theme:{background:'#20252e',foreground:'#f0f0f0'},screenReaderMode:false});
const fit = new FitAddon.FitAddon();
terminal.loadAddon(fit);
terminal.open(document.getElementById('terminal'));
let homePath=null, selectedFolder=null, selectedChoice=null, availableChoices=[], folders=[], expandedFolders=false, selectingFolder=false;
let started=false, starting=false, generic=false, input=Promise.resolve(), resizeTimer;
function report(e){status.textContent=String(e);}
function measure(){if(innerWidth<20||innerHeight<60)return;fit.fit();if(started)invoke('terminal_resize',{cols:terminal.cols,rows:terminal.rows}).catch(report);}
new ResizeObserver(()=>{clearTimeout(resizeTimer);resizeTimer=setTimeout(measure,60);}).observe(document.getElementById('terminal'));
function sendInput(data){input=input.then(()=>invoke('terminal_write',{data})).catch(report);}
terminal.onData(sendInput);
window.installTerminalIme(terminal, sendInput);
// xterm owns composition events and Ctrl+C. Copy/paste uses local clipboard only.
terminal.attachCustomKeyEventHandler(e=>{
  if(e.isComposing || e.keyCode === 229) return true;
  const mac=navigator.platform.includes('Mac');
  const copy=(mac?e.metaKey:e.ctrlKey&&e.shiftKey)&&e.key.toLowerCase()==='c';
  const paste=(mac?e.metaKey:e.ctrlKey&&e.shiftKey)&&e.key.toLowerCase()==='v';
  if(!copy&&!paste)return true;
  if(e.type==='keydown'){
    e.preventDefault();
    if(copy){const text=terminal.getSelection();if(text)window.__TAURI__.clipboardManager.writeText(text).catch(report);}
    else window.__TAURI__.clipboardManager.readText().then(text=>{if(text)terminal.paste(text);}).catch(report);
  }
  return false;
});
// Focus the real xterm textarea on an explicit content click; never steal
// application focus on startup or interrupt an active IME composition.
document.getElementById('terminal').addEventListener('pointerdown', e => {
  if(e.button === 0) terminal.focus();
});
document.querySelector('header').addEventListener('pointerdown',e=>{if(e.button===0&&!e.target.closest('button'))win.startDragging().catch(report);});
document.querySelectorAll('.resize').forEach(handle=>handle.addEventListener('pointerdown',e=>{if(e.button===0)win.startResizeDragging(handle.dataset.direction).catch(report);}));
document.getElementById('close').onclick=()=>invoke(generic?'terminal_close':'terminal_hide').catch(report);
function setInfo(info){
  document.getElementById('title').textContent=info.title;

}
async function start(choice, trigger){
  if(started||starting)return;
  starting=true;
  status.textContent=`${trigger?.dataset?.label||'터미널'} 시작 중…`;
  document.getElementById('welcome').setAttribute('aria-busy','true');
  document.getElementById('welcome').hidden=true;
  document.getElementById('terminal').hidden=false;measure();
  const startedAt=clock(); let firstOutput=true;
  const output=new Channel();output.onmessage=bytes=>{
    const measure=window.gibbonMeasure;
    if(firstOutput){firstOutput=false;measure?.('terminal.first_output',clock()-startedAt);}
    if(measure){
      const receivedAt=clock();
      terminal.write(new Uint8Array(bytes),()=>measure('terminal.output.parse',clock()-receivedAt));
    }else terminal.write(new Uint8Array(bytes));
  };
  try{
    if(generic && selectedFolder)await invoke('terminal_folder',{path:selectedFolder.path});
    await (window.gibbonLocalFlow || ((_op, work) => work({invoke, stage:()=>{}})))('terminal.start', async flow => {
      await flow.invoke('terminal_start',{cols:terminal.cols,rows:terminal.rows,output,choice});
      flow.stage('response');
      flow.frame?.();
    });
    started=true;status.textContent='';measure();
    if(trigger && document.hasFocus() && (document.activeElement===trigger || document.activeElement===document.body))terminal.focus();
  }catch(e){
    const restoreFocus=trigger && document.hasFocus() && (document.activeElement===trigger || document.activeElement===document.body);
    report(e);if(generic){document.getElementById('welcome').hidden=false;document.getElementById('terminal').hidden=true;if(restoreFocus)trigger.focus({preventScroll:true});}
  }
  finally{starting=false;document.getElementById('welcome').setAttribute('aria-busy','false');}
}
function selectionChanged(){
  document.getElementById('home-hint').hidden=selectedFolder?.path!==homePath;
  for(const button of document.getElementById('recommendations').children)button.setAttribute('aria-pressed',String(button.dataset.choice===selectedChoice));
  for(const button of document.getElementById('recent-folders').children)button.setAttribute('aria-pressed',String(button.dataset.path===selectedFolder?.path));
  const choice=availableChoices.find(item=>item.id===selectedChoice);
  document.getElementById('command-preview').textContent=selectedFolder && choice ? `${selectedFolder.command}${navigator.platform.includes('Win')?' -ErrorAction Stop':' &&'}\n${choice.command}` : '';
  document.getElementById('run-task').disabled=selectingFolder || !choice;
}
function renderFolders(){
  const list=document.getElementById('recent-folders');list.replaceChildren();
  for(const folder of folders.slice(0,expandedFolders?folders.length:3)){
    const button=document.createElement('button');button.className='select-row';button.dataset.path=folder.path;button.title=folder.path;
    button.setAttribute('aria-label',folder.path);
    const name=document.createElement('span');name.className='folder-name';name.textContent=folder.name;
    const parent=document.createElement('span');parent.className='folder-parent';parent.textContent=folder.parent;
    button.append(name,parent);button.onclick=()=>{selectedFolder=folder;selectionChanged();};list.append(button);
  }
  const more=document.getElementById('more-folders');more.hidden=folders.length<=3;more.textContent=expandedFolders?'접기':'더 보기…';more.setAttribute('aria-expanded',String(expandedFolders));
  selectionChanged();
}
async function welcome(){
  const data=await invoke('terminal_welcome');setInfo(data.info);
  availableChoices=data.choices;folders=data.folders||[];selectedFolder=data.location;homePath=data.home;
  if(data.info.last_path && selectedFolder){folders=[selectedFolder,...folders.filter(f=>f.path!==selectedFolder.path)].slice(0,20);}
  document.getElementById('home-hint').hidden=Boolean(data.info.last_path);
  if(!availableChoices.some(item=>item.id===selectedChoice))selectedChoice=availableChoices[0]?.id;
  const list=document.getElementById('recommendations');list.replaceChildren();
  for(const item of availableChoices){
    const button=document.createElement('button');button.className='select-row';button.dataset.choice=item.id;button.textContent=item.label;
    button.onclick=()=>{selectedChoice=item.id;selectionChanged();};list.append(button);
  }
  renderFolders();
}
document.getElementById('more-folders').onclick=()=>{expandedFolders=!expandedFolders;renderFolders();};
document.getElementById('run-task').onclick=()=>{if(!selectingFolder) return start(selectedChoice,document.getElementById('run-task'));};
document.getElementById('choose-folder').onclick=async()=>{
  if(selectingFolder)return;selectingFolder=true;selectionChanged();
  try{const result=await invoke('terminal_folder',{path:null});if(result.folder_selected!==false)await welcome();status.textContent='';}catch(e){report(e);}finally{selectingFolder=false;selectionChanged();}
};
document.getElementById('rename-form').onsubmit=async event=>{event.preventDefault();try{setInfo(await invoke('terminal_rename',{title:document.getElementById('rename-title').value}));status.textContent='창 제목을 변경했습니다.';}catch(e){report(e);}};
(async()=>{
  const info=await invoke('terminal_info');generic=info.kind==='terminal';setInfo(info);
  if(generic){
    const close=document.getElementById('close');close.title='터미널 종료';close.setAttribute('aria-label','터미널 종료');
    document.getElementById('welcome').hidden=false;document.getElementById('terminal').hidden=true;
    await welcome();
  }else{await start(null);}
  window.gibbonMeasure?.('terminal.ui.ready',clock());
})().catch(report);
// Refresh only the current directory; never collect input, output or history.
setInterval(()=>{if(generic&&started)invoke('terminal_info').then(setInfo).catch(report);},2000);

document.getElementById('pin').onclick=async()=>{
  try { await invoke('toggle_pin'); const button=document.getElementById('pin'); button.setAttribute('aria-pressed',String(button.getAttribute('aria-pressed')!=='true')); } catch(e){report(e);}
};
document.getElementById('font-size').onclick=()=>{
  const sizes=[13,14,16,18];terminal.options.fontSize=sizes[(sizes.indexOf(terminal.options.fontSize)+1)%sizes.length];
  const button=document.getElementById('font-size');button.title=`글자 크기 ${terminal.options.fontSize}px · 클릭하여 변경`;button.setAttribute('aria-label',button.title);measure();
};
document.getElementById('copy').onclick=()=>{
  const text=terminal.getSelection();if(text)window.__TAURI__.clipboardManager.writeText(text).catch(report);
};
