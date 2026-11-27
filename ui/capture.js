'use strict';
const invoke = window.jajanInvoke || window.__TAURI__.core.invoke;
const selection = document.getElementById('selection');
const shade = document.getElementById('drag-shade');
const hint = document.getElementById('hint');
for (const { x, y, width, height } of window.captureNoteFrames || []) {
  const outline = document.createElement('div');
  Object.assign(outline.style, {
    left: `${x}px`, top: `${y}px`, width: `${width}px`, height: `${height}px`
  });
  document.getElementById('note-outlines').append(outline);
}
let start, rect, ready = true, busy = false;
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
  const { x, y, width, height } = rect;
  if (width < 2 || height < 2) {
    selection.hidden = true;
    shade.classList.remove('active');
    return;
  }
  shade.style.clipPath = `polygon(evenodd, 0 0, 100% 0, 100% 100%, 0 100%, 0 0, ${x}px ${y}px, ${x}px ${y+height}px, ${x+width}px ${y+height}px, ${x+width}px ${y}px, ${x}px ${y}px)`;
  shade.classList.add('active');
  selection.hidden = false;

});
document.addEventListener('pointerup', async () => {
  if (!start || busy) return; start = null;
  if (!rect || rect.width < 2 || rect.height < 2) return;
  busy = true;
  try { await invoke('finish_capture',rect); }
  catch(error) { hint.hidden = false; hint.textContent = String(error); busy = false; }
});
window.addEventListener('keydown', e => {
  if (e.key !== 'Escape' || busy) return;
  e.preventDefault();
  busy = true;
  const pointerId = start?.pointerId;
  start = null;
  rect = null;
  selection.hidden = true;
  shade.classList.remove('active');
  hint.hidden = true;
  if (pointerId !== undefined && document.body.hasPointerCapture(pointerId)) {
    document.body.releasePointerCapture(pointerId);
  }
  invoke('cancel_capture').catch(error => {
    busy = false;
    hint.hidden = false;
    hint.textContent = String(error);
  });
}, true);
document.addEventListener('contextmenu',e => e.preventDefault());
// The native side can show and focus this overlay as soon as it receives ready.
invoke('capture_ready').catch(() => {
  ready = false;
  invoke('capture_failed').catch(() => {});
});
