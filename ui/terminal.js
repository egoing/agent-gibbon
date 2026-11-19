'use strict';
const { Channel } = window.__TAURI__.core;
const invoke = window.jajanInvoke || window.__TAURI__.core.invoke;
const win = window.__TAURI__.window.getCurrentWindow();
const status = document.getElementById('status');
const clock = () => window.performance?.now?.() || 0;
const terminal = new Terminal({fontSize:window.jajanContentSize.defaultValue('terminal'),fontFamily:'Menlo, Monaco, monospace',cursorBlink:true,scrollback:3000,allowProposedApi:false,theme:{background:'#20252e',foreground:'#f0f0f0'},screenReaderMode:false});
const fit = new FitAddon.FitAddon();
terminal.loadAddon(fit);
terminal.open(document.getElementById('terminal'));
let homePath=null, selectedFolder=null, selectedChoice=null, availableChoices=[], folders=[], expandedFolders=false, selectingFolder=false;
let restoreSupported=true;
let ended=false;
let started=false, starting=false, input=Promise.resolve(), resizeTimer;
function report(e){status.textContent=String(e);}
// A successful terminal_start creates the PTY at the supplied cell size. Only
// one subsequent resize may be in flight; xterm holds the latest desired grid.
let appliedSize=null, resizing=false, retryTimer;
async function syncPtySize(retry=true){
  if(!started || ended || resizing)return;
  clearTimeout(retryTimer);
  resizing=true;
  try{
    while(!ended && (terminal.cols!==appliedSize.cols || terminal.rows!==appliedSize.rows)){
      const size={cols:terminal.cols,rows:terminal.rows};
      try{
        await invoke('terminal_resize',size);
        appliedSize=size;
      }catch(error){
        if(ended)break;
        report(error);
        if(size.cols!==terminal.cols || size.rows!==terminal.rows)continue;
        if(retry)retryTimer=setTimeout(()=>syncPtySize(false),120);
        break;
      }
    }
  }finally{resizing=false;}
}
function measure(){
  const host=document.getElementById('terminal');
  if(!host.clientWidth||!host.clientHeight)return;
  // FitAddon reserves a native scrollbar column. Our thumb overlays the content.
  // Use the rendered public screen geometry to fit complete cells only.
  const screen=host.querySelector('.xterm-screen');
  const cellWidth=parseFloat(getComputedStyle(screen).width)/terminal.cols, cellHeight=parseFloat(getComputedStyle(screen).height)/terminal.rows;
  const css=getComputedStyle(host);
  const width=host.clientWidth-parseFloat(css.paddingLeft)-parseFloat(css.paddingRight);
  const height=host.clientHeight-parseFloat(css.paddingTop)-parseFloat(css.paddingBottom);
  const forced=matchMedia('(forced-colors: active)').matches;
  if(forced)fit.fit();
  else if(cellWidth>0&&cellHeight>0)terminal.resize(Math.max(2,Math.floor(width/cellWidth)),Math.max(1,Math.floor(height/cellHeight)));
  void syncPtySize();
}
function scheduleMeasure(){
  clearTimeout(resizeTimer);
  resizeTimer=setTimeout(measure,60);
}
new ResizeObserver(scheduleMeasure).observe(document.getElementById('terminal'));
document.addEventListener('visibilitychange',()=>{if(!document.hidden)scheduleMeasure();});
function sendInput(data){input=input.then(()=>{if(!ended)return invoke('terminal_write',{data});}).catch(report);}
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
document.querySelectorAll('.resize').forEach(handle=>handle.addEventListener('pointerdown',e=>{if(e.button===0)win.startResizeDragging(handle.dataset.direction).catch(report);}));
document.getElementById('close').onclick=()=>invoke('terminal_close').catch(report);
function setInfo(info){
  ended ||= info.exit_code!=null;
  document.getElementById('title').textContent=info.running && info.program && info.program !== 'shell' ? info.program : info.title;

}
async function start(choice, trigger, plan, execRequest){
  if(started||starting||ended)return;
  if(!restoreSupported && (plan !== undefined || choice === 'auto-monitor')){
    report('터미널 복원 실행은 이 운영체제에서 아직 지원하지 않습니다. 현재 macOS에서만 사용할 수 있습니다.');
    return;
  }
  starting=true;
  status.textContent=`${trigger?.dataset?.label||'터미널'} 시작 중…`;
  document.getElementById('welcome').setAttribute('aria-busy','true');
  document.getElementById('welcome').hidden=true;
  document.getElementById('terminal').hidden=false;measure();
  const startedAt=clock(); let firstOutput=true;
  const output=new Channel();output.onmessage=bytes=>{
    const measure=window.jajanMeasure;
    if(firstOutput){firstOutput=false;measure?.('terminal.first_output',clock()-startedAt);}
    if(measure){
      const receivedAt=clock();
      terminal.write(new Uint8Array(bytes),()=>measure('terminal.output.parse',clock()-receivedAt));
    }else terminal.write(new Uint8Array(bytes));
  };
  try{
    if(selectedFolder)await invoke('terminal_folder',{path:selectedFolder.path});
    await (window.jajanLocalFlow || ((_op, work) => work({invoke, stage:()=>{}})))('terminal.start', async flow => {
      const size={cols:terminal.cols,rows:terminal.rows};
      await flow.invoke('terminal_start',{...size,output,choice,...(execRequest === undefined ? {} : {execRequest}),...(plan === undefined ? {} : {plan})});
      appliedSize=size;
      flow.stage('response');
      flow.frame?.();
    });
    started=true;status.textContent='';measure();
    if(trigger && document.hasFocus() && (document.activeElement===trigger || document.activeElement===document.body))terminal.focus();
  }catch(e){
    const restoreFocus=trigger && document.hasFocus() && (document.activeElement===trigger || document.activeElement===document.body);
    report(e);{document.getElementById('welcome').hidden=false;document.getElementById('terminal').hidden=true;if(restoreFocus)trigger.focus({preventScroll:true});}
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
const restorePanel=document.getElementById('restore-panel');
const welcomePanel=document.getElementById('welcome-panel');
const planInput=document.getElementById('restore-plan');
const restoreRun=document.getElementById('restore-run');
restoreRun.onclick=()=>start('shell',restoreRun,planInput.value);
planInput.oninput=()=>{restoreRun.disabled=!planInput.value.trim();};
document.getElementById('restore-cancel').onclick=()=>{
  restorePanel.hidden=true;
  welcomePanel.hidden=false;
  document.getElementById('choose-folder').focus();
};
document.getElementById('more-folders').onclick=()=>{expandedFolders=!expandedFolders;renderFolders();};
document.getElementById('run-task').onclick=()=>{if(!selectingFolder) return start(selectedChoice,document.getElementById('run-task'));};
document.getElementById('choose-folder').onclick=async()=>{
  if(selectingFolder)return;selectingFolder=true;selectionChanged();
  try{const result=await invoke('terminal_folder',{path:null});if(result.folder_selected!==false)await welcome();status.textContent='';}catch(e){report(e);}finally{selectingFolder=false;selectionChanged();}
};
async function initializeTerminal() {
  const info=await invoke('terminal_info');
  if(info.kind!=='terminal')throw Error('전용 에이전트 기능은 지원하지 않습니다.');
  restoreSupported=info.restore_supported !== false;
  setInfo(info);
  {
    const close=document.getElementById('close');close.title='터미널 종료';close.setAttribute('aria-label','터미널 종료');
    if(ended){
      document.getElementById('welcome').hidden=true;
      document.getElementById('terminal').hidden=false;
      terminal.write('\r\n[PTY 세션 종료]\r\n');
      return;
    }
    document.getElementById('welcome').hidden=false;document.getElementById('terminal').hidden=true;
    await welcome();
    if(info.restore_plan && !info.running){planInput.value=info.restore_plan;restorePanel.hidden=false;welcomePanel.hidden=true;
      if(!restoreSupported)report('터미널 복원 실행은 이 운영체제에서 아직 지원하지 않습니다. 현재 macOS에서만 사용할 수 있습니다.');
      else if(info.restored && info.auto_restore) await start('auto-monitor', null);
    }
    else if(info.restored) status.textContent = `이전 터미널 프로세스는 종료되었습니다. ${info.last_command ? `마지막 명령: ${info.last_command} · 자동 실행하지 않았습니다.` : '마지막 명령은 안전한 복원을 위해 저장에서 제외되었습니다.'}`;
  }
  window.jajanMeasure?.('terminal.ui.ready',clock());
}
if (window.jajanRegionFrame) window.jajanRegionFrame.prepare(initializeTerminal);
else initializeTerminal().catch(report);
// Refresh only the current directory; never collect input, output or history.
setInterval(()=>{if(started && !ended)invoke('terminal_info').then(setInfo).catch(report);},2000);

document.getElementById('pin').onclick=async()=>{
  try { await invoke('toggle_pin'); const button=document.getElementById('pin'); button.setAttribute('aria-pressed',String(button.getAttribute('aria-pressed')!=='true')); } catch(e){report(e);}
};
window.jajanContentSize.bind('terminal', document.getElementById('font-size'), size => {
  terminal.options.fontSize = size;
  scheduleMeasure();
}, report);
