'use strict';
const invoke = window.gibbonInvoke || window.__TAURI__.core.invoke;
const selection = document.getElementById('selection');
const hint = document.getElementById('hint');
const shade = document.getElementById('shade');
// Each display receives this after its own native window is ordered front.
// Pointer and Escape handlers never wait for the visual transition.
window.addEventListener('capture-shown', () => {
  requestAnimationFrame(() => requestAnimationFrame(() => {
    document.documentElement.classList.add('mask-shown');
  }));
}, { once: true });
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
  start = { ...point(e), pointerId: e.pointerId }; rect = null; document.body.setPointerCapture(e.pointerId);
});
document.addEventListener('pointermove', e => {
  if (!start || busy) return;
  const end = point(e);
  rect = { x:Math.min(start.x,end.x), y:Math.min(start.y,end.y), width:Math.abs(end.x-start.x), height:Math.abs(end.y-start.y) };
  Object.assign(selection.style,{left:`${rect.x}px`,top:`${rect.y}px`,width:`${rect.width}px`,height:`${rect.height}px`});
  selection.hidden = false;
  const {x, y, width, height} = rect;
  // Cut the selected rectangle out of the existing dimming layer, retaining
  // its current opacity even when a drag begins during the transition.
  shade.style.clipPath = `polygon(evenodd, 0 0, 100% 0, 100% 100%, 0 100%, 0 0, ${x}px ${y}px, ${x}px ${y+height}px, ${x+width}px ${y+height}px, ${x+width}px ${y}px, ${x}px ${y}px)`;
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
  if (e.key !== 'Escape' || busy) return;
  e.preventDefault();
  busy = true;
  if (start && document.body.hasPointerCapture(start.pointerId)) {
    document.body.releasePointerCapture(start.pointerId);
  }
  start = null;
  rect = null;
  invoke('cancel_capture').catch(error => {
    busy = false;
    selection.hidden = true;
    shade.style.clipPath = '';
    hint.textContent = String(error);
  });
});
document.addEventListener('contextmenu',e => e.preventDefault());
