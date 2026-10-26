'use strict';
const { invoke } = window.__TAURI__.core;
const selection = document.getElementById('selection');
const hint = document.getElementById('hint');
let start, rect, ready = false, busy = false;
// Native pixels are already decoded. Draw once while hidden; do not wait on
// image loading, base64 conversion, or PNG compression before showing selection.
invoke('capture_screen').then(async bytes => {
  const header = new DataView(bytes);
  const width = header.getUint32(0, true), height = header.getUint32(4, true);
  if (!width || !height || bytes.byteLength !== 8 + width * height * 4) throw new Error('Invalid preview');
  const canvas = document.getElementById('screen');
  canvas.width = width; canvas.height = height;
  const context = canvas.getContext('2d', { alpha: false });
  if (!context) throw new Error('Canvas unavailable');
  context.putImageData(new ImageData(new Uint8ClampedArray(bytes, 8), width, height), 0, 0);
  ready = true;
  await invoke('capture_ready');
}).catch(() => {
  ready = false;
  invoke('capture_failed').catch(() => {});
});
const point = e => ({ x: Math.max(0,Math.min(innerWidth,e.clientX)), y:Math.max(0,Math.min(innerHeight,e.clientY)) });
document.addEventListener('pointerdown', e => {
  if (!ready || busy || e.button !== 0) return;
  start = point(e); rect = null; document.body.setPointerCapture(e.pointerId);
});
document.addEventListener('pointermove', e => {
  if (!start || busy) return;
  const end = point(e);
  rect = { x:Math.min(start.x,end.x), y:Math.min(start.y,end.y), width:Math.abs(end.x-start.x), height:Math.abs(end.y-start.y) };
  Object.assign(selection.style,{left:`${rect.x}px`,top:`${rect.y}px`,width:`${rect.width}px`,height:`${rect.height}px`});
  selection.hidden = false; document.getElementById('shade').hidden = true;
  hint.textContent = `${Math.round(rect.width)} × ${Math.round(rect.height)} · Escape 취소`;
});
document.addEventListener('pointerup', async () => {
  if (!start || busy) return; start = null;
  if (!rect || rect.width < 2 || rect.height < 2) return;
  busy = true;
  try { await invoke('finish_capture',rect); }
  catch(error) { hint.textContent = `${error} · Escape로 닫기`; busy = false; }
});
document.addEventListener('keydown', e => {
  if (e.key === 'Escape' && !busy) invoke('cancel_capture').catch(error => { hint.textContent = String(error); });
});
document.addEventListener('contextmenu',e => e.preventDefault());
