'use strict';
import { themeFor } from './theme.mjs';
const { invoke } = window.__TAURI__.core;
const win = window.__TAURI__.window.getCurrentWindow();
const $ = (id) => document.getElementById(id);
let revision = -1;
function failure(error) { $('error').textContent = String(error); $('error').hidden = false; }
async function refresh() {
  try {
    const state = await invoke('snapshot');
    if (revision !== state.note.revision) {
      const theme = themeFor(state.note.color);
      document.documentElement.style.setProperty('--background', state.note.color);
      document.documentElement.style.setProperty('--foreground', theme.foreground);
      document.documentElement.style.colorScheme = theme.scheme;
      document.documentElement.style.setProperty('--border', theme.border);
      document.documentElement.style.setProperty('--panel', theme.panel);
      $('viewport').setAttribute('aria-label', state.note.title || '메모 내용');
      // HTML is generated and sanitized in Rust; no user HTML reaches this sink.
      $('content').innerHTML = state.note.html;
      revision = state.note.revision;
    }
    $('pin').setAttribute('aria-pressed', String(state.always_on_top));
    $('error').hidden = true;
  } catch (error) { failure(error); }
  setTimeout(refresh, 350);
}
document.addEventListener('mousedown', (event) => {
  if (event.button !== 0 || event.target.closest('button, .resize')) return;
  event.preventDefault();
  win.startDragging().catch(failure);
});
for (const grip of document.querySelectorAll('.resize')) {
  grip.addEventListener('mousedown', (event) => { if (event.button === 0) win.startResizeDragging(grip.dataset.direction).catch(failure); });
}
$('pin').addEventListener('click', () => invoke('toggle_pin').catch(failure));
$('close').addEventListener('click', () => invoke('hide_note').catch(failure));
document.addEventListener('keydown', (event) => { if (event.key === 'Escape') invoke('hide_note').catch(failure); });
let copyFeedback;
$('copy').addEventListener('click', async () => {
  try {
    await window.__TAURI__.clipboardManager.writeText($('content').innerText);
    $('copy').dataset.copied = 'true';
    $('copy').title = '복사됨';
    clearTimeout(copyFeedback);
    copyFeedback = setTimeout(() => {
      delete $('copy').dataset.copied;
      $('copy').title = '본문 복사';
    }, 1200);
  } catch (error) { failure(error); }
});
$('content').addEventListener('click', (event) => { if (event.target.closest('a')) event.preventDefault(); });
refresh();
