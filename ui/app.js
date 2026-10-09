'use strict';
const { invoke } = window.__TAURI__.core;
const win = window.__TAURI__.window.getCurrentWindow();
const $ = (id) => document.getElementById(id);
let revision = -1;
function failure(error) { $('error').textContent = String(error); $('error').hidden = false; }
async function refresh() {
  try {
    const state = await invoke('snapshot');
    if (revision !== state.note.revision) {
      $('title').textContent = state.note.title;
      // HTML is generated and sanitized in Rust; no user HTML reaches this sink.
      $('content').innerHTML = state.note.html;
      $('status').textContent = state.note.status;
      $('updated').textContent = state.note.updated_at ? new Date(state.note.updated_at).toLocaleTimeString('ko-KR') + ' 갱신' : '';
      revision = state.note.revision;
    }
    $('pin').setAttribute('aria-pressed', String(state.always_on_top));
    $('error').hidden = true;
  } catch (error) { failure(error); }
  setTimeout(refresh, 350);
}
$('drag').addEventListener('mousedown', (event) => {
  if (event.button === 0 && !event.target.closest('button')) win.startDragging().catch(failure);
});
for (const grip of document.querySelectorAll('.resize')) {
  grip.addEventListener('mousedown', (event) => { if (event.button === 0) win.startResizeDragging(grip.dataset.direction).catch(failure); });
}
$('pin').addEventListener('click', () => invoke('toggle_pin').catch(failure));
$('close').addEventListener('click', () => invoke('hide_note').catch(failure));
document.addEventListener('keydown', (event) => { if (event.key === 'Escape') invoke('hide_note').catch(failure); });
$('content').addEventListener('click', (event) => { if (event.target.closest('a')) event.preventDefault(); });
refresh();
