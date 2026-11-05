'use strict';
const { invoke, Channel } = window.__TAURI__.core;
const win = window.__TAURI__.window.getCurrentWindow();
const status = document.getElementById('status');
const terminal = new Terminal({fontSize:13,fontFamily:'Menlo, Monaco, monospace',cursorBlink:true,scrollback:3000,allowProposedApi:false,theme:{background:'#20252e',foreground:'#f0f0f0'},screenReaderMode:false});
const fit = new FitAddon.FitAddon();
terminal.loadAddon(fit);
terminal.open(document.getElementById('terminal'));
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
  const path=document.getElementById('work-path');path.hidden=!info.last_path;path.textContent=info.last_path||'';
}
async function start(choice){
  if(started||starting)return;
  starting=true;
  document.getElementById('welcome').hidden=true;
  document.getElementById('terminal').hidden=false;measure();
  const output=new Channel();output.onmessage=bytes=>terminal.write(new Uint8Array(bytes));
  try{
    await invoke('terminal_start',{cols:terminal.cols,rows:terminal.rows,output,choice});
    started=true;status.textContent='';measure();
  }catch(e){report(e);if(generic){document.getElementById('welcome').hidden=false;document.getElementById('terminal').hidden=true;}}
  finally{starting=false;}
}
async function welcome(){
  const {info,choices}=await invoke('terminal_welcome');setInfo(info);
  const list=document.getElementById('recommendations');list.replaceChildren();
  for(const item of choices){const button=document.createElement('button');button.textContent=item.label;button.title=item.command;button.onclick=()=>start(item.id);list.append(button);}
  document.getElementById('commands').textContent=choices.map(item=>`${item.label}: ${item.command}`).join('\n');
}
document.getElementById('open-shell').onclick=()=>start('shell');
document.getElementById('choose-folder').onclick=async()=>{try{setInfo(await invoke('terminal_folder'));}catch(e){report(e);}};
document.getElementById('rename-form').onsubmit=async event=>{event.preventDefault();try{setInfo(await invoke('terminal_rename',{title:document.getElementById('rename-title').value}));}catch(e){report(e);}};
(async()=>{
  const info=await invoke('terminal_info');generic=info.kind==='terminal';setInfo(info);
  if(generic){
    const close=document.getElementById('close');close.title='터미널 종료';close.setAttribute('aria-label','터미널 종료');
    document.getElementById('welcome').hidden=false;document.getElementById('terminal').hidden=true;
    await welcome();
  }else{await start(null);}
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
