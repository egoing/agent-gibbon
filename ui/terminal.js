'use strict';
const { invoke, Channel } = window.__TAURI__.core;
const win = window.__TAURI__.window.getCurrentWindow();
const status = document.getElementById('status');
const terminal = new Terminal({fontSize:13,fontFamily:'Menlo, Monaco, monospace',cursorBlink:true,scrollback:3000,allowProposedApi:false,theme:{background:'#20252e',foreground:'#f0f0f0'},screenReaderMode:false});
const fit = new FitAddon.FitAddon();
terminal.loadAddon(fit);
terminal.open(document.getElementById('terminal'));
let started=false, input=Promise.resolve(), resizeTimer;
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
document.getElementById('close').onclick=()=>invoke('terminal_hide').catch(report);
(async()=>{
  const info=await invoke('terminal_info');
  document.getElementById('title').textContent=`에이전트 터미널 · ${info.id}`;
  measure();
  const output=new Channel();
  output.onmessage=bytes=>terminal.write(new Uint8Array(bytes));
  const session=await invoke('terminal_start',{cols:terminal.cols,rows:terminal.rows,output});
  started=true;status.textContent='';measure();
  document.getElementById('title').textContent=`${session.program} · ${session.id}`;
})().catch(report);

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
