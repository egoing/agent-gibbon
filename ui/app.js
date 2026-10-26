'use strict';
import { themeFor } from './theme.mjs';
const { invoke } = window.__TAURI__.core;
const win = window.__TAURI__.window.getCurrentWindow();
const $ = (id) => document.getElementById(id);
let revision = -1;
let latestNote;
let displayedHtml;
let highlightRevision = null;
let acknowledging = false;
let acknowledgedRevision = -1;
let editing = false;
let initialEditHandled = false;
let saving = false;
let editRevision;
let needsFit = true;
let fitting = false;
let folded = false;
async function fitContent() {
  if (latestNote?.capture || !needsFit || fitting || editing || folded) return;
  fitting = true;
  // Measure natural flow independently of the current window height.
  const measure = $('viewport').cloneNode(true);
  measure.id = 'measure-viewport';
  Object.assign(measure.style, {
    position: 'fixed', left: '-10000px', top: '0', visibility: 'hidden',
    display: 'flow-root', flex: 'none', width: '298px', minWidth: '298px',
    maxWidth: '298px', height: 'auto', minHeight: '0', maxHeight: 'none',
    overflow: 'visible', padding: '3px', overflowWrap: 'anywhere', pointerEvents: 'none',
  });
  document.body.appendChild(measure);
  const rect = measure.getBoundingClientRect();
  const width = Math.ceil(rect.width + 2);
  const height = Math.ceil(rect.height + $('diff-actions').getBoundingClientRect().height + 2);
  measure.remove();
  try { needsFit = !await invoke('fit_note', { width, height }); }
  catch (error) { failure(error); }
  finally { fitting = false; }
}
const fontSizes = [12, 14, 16, 18];
let fontIndex = 0;
$('font-size').addEventListener('click', () => {
  fontIndex = (fontIndex + 1) % fontSizes.length;
  const size = fontSizes[fontIndex];
  document.documentElement.style.setProperty('--font-size', `${size}px`);
  needsFit = true;
  fitContent();
  $('font-size').title = `글자 크기 ${size}px · 클릭하여 변경`;
  $('font-size').setAttribute('aria-label', `글자 크기 ${size}px, 클릭하여 변경`);
});
function endEditing() {
  editing = false;
  $('editor').hidden = true;
  $('content').hidden = false;
  $('copy').disabled = false;
  $('edit').disabled = false;
  $('error').hidden = true;
  needsFit = true;
  fitContent();
  $('edit').focus();
}
function beginEditing() {
  if (!latestNote || editing) return;
  editing = true;
  editRevision = latestNote.revision;
  $('markdown').value = latestNote.markdown;
  $('save-target').textContent = latestNote.source_path ? '원본 파일에 저장' : '메모에 저장';
  $('save-target').title = latestNote.source_path || '앱 종료 시 초기화됩니다';
  $('content').hidden = true;
  $('editor').hidden = false;
  $('copy').disabled = true;
  $('edit').disabled = true;
  $('error').hidden = true;
  $('markdown').focus();
}
$('edit').addEventListener('click', beginEditing);
$('cancel').addEventListener('click', endEditing);
async function save() {
  if (!editing || saving) return;
  saving = true;
  $('save').disabled = true;
  $('cancel').disabled = true;
  $('markdown').readOnly = true;
  try {
    latestNote = await invoke('save_note', { markdown: $('markdown').value, expectedRevision: editRevision });
    highlightRevision = null;
    acknowledgedRevision = latestNote.revision;
    $('clear-diff').hidden = true;
    $('diff-actions').hidden = true;
    // Direct saves confirm the content; subsequent external revisions can highlight again.
    $('content').innerHTML = latestNote.html;
    displayedHtml = $('content').innerHTML;
    revision = latestNote.revision;
    endEditing();
  } catch (error) { failure(error); }
  finally {
    saving = false;
    $('save').disabled = false;
    $('cancel').disabled = false;
    $('markdown').readOnly = false;
  }
}
$('save').addEventListener('click', save);
function failure(error) { $('error').textContent = String(error); $('error').hidden = false; }
async function refresh() {
  try {
    const state = await invoke('snapshot');
    latestNote = state.note;
    document.body.classList.toggle('pointer-over', !!state.hovered);
    $('dev-instance').hidden = !state.dev_label;
    $('dev-instance').textContent = state.dev_label || '';
    warpTipKey = state.instance ? `floatnote.warp-tip-dismissed:${state.instance}` : 'floatnote.warp-tip-dismissed';
    $('fold-title').textContent = state.dev_label ? `[${state.dev_label}] ${state.note.title}` : state.note.title;
    $('fold-title').title = state.note.title;
    $('edit').disabled = editing || !!state.note.capture;
    $('font-size').disabled = !!state.note.capture;
    $('copy').disabled = editing;
    $('copy').title = $('copy').dataset.copied === 'true'
      ? (state.note.capture ? '이미지 복사됨' : '복사됨')
      : (state.note.capture ? '이미지 복사' : '본문 복사');
    $('copy').setAttribute('aria-label', $('copy').title);
    document.body.classList.toggle('image-note', !!state.note.capture);
    if (revision !== state.note.revision) {
      const theme = themeFor(state.note.color);
      document.documentElement.style.setProperty('--background', state.note.color);
      document.documentElement.style.setProperty('--foreground', theme.foreground);
      document.documentElement.style.colorScheme = theme.scheme;
      document.documentElement.style.setProperty('--border', theme.border);
      document.documentElement.style.setProperty('--panel', theme.panel);
      $('viewport').setAttribute('aria-label', state.note.title || '메모 내용');
      // HTML is generated and sanitized in Rust; no user HTML reaches this sink.
      // Feedback rendering is synchronized below even when revision is unchanged.
      revision = state.note.revision;
      needsFit = true;
    }
    highlightRevision = (state.highlight_revision ?? -1) > acknowledgedRevision ? state.highlight_revision : null;
    $('clear-diff').hidden = highlightRevision === null;
    $('diff-actions').hidden = highlightRevision === null;
    $('clear-diff').disabled = acknowledging;
    const html = highlightRevision !== null ? state.note.change_html : state.note.html;
    if (displayedHtml !== html) {
      $('content').innerHTML = html;
      if (state.note.capture) {
        const img = document.createElement('img');
        img.alt = '화면 캡처'; img.draggable = false;
        img.src = await invoke('note_image');
        $('content').replaceChildren(img);
      }
      displayedHtml = html;
    }
    $('pin').setAttribute('aria-pressed', String(state.always_on_top));
    if (!initialEditHandled) {
      initialEditHandled = true;
      if (state.note.edit_on_open && !state.note.capture) beginEditing();
    }
    await fitContent();
  } catch (error) { failure(error); }
  setTimeout(refresh, 350);
}
document.addEventListener('mousedown', (event) => {
  if (folded || event.button !== 0 || event.target.closest('button, #editor, #error, #warp-tip, #diff-actions, .resize')) return;
  event.preventDefault();
  invoke('drag_note').catch(failure);
});
for (const grip of document.querySelectorAll('.resize')) {
  grip.addEventListener('mousedown', (event) => {
    if (event.button !== 0 || folded) return;
    needsFit = false;
    // Start native resizing inside the original mouse gesture.
    invoke('manual_resize').catch(failure);
    win.startResizeDragging(grip.dataset.direction).catch(failure);
  });
}
$('pin').addEventListener('click', () => invoke('toggle_pin').catch(failure));
$('close').addEventListener('click', () => invoke('hide_note').catch(failure));
document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape') {
    event.preventDefault();
    if (editing) { if (!saving) endEditing(); }
    else invoke('hide_note').catch(failure);
  }
  if (!editing && latestNote && (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'c') {
    event.preventDefault();
    copyNote();
  }
  if (editing && (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
    event.preventDefault();
    save();
  }
});
let copyFeedback;
// Render the current sanitized HTML offscreen: innerText retains the existing
// rendered-text format without ever touching the visible diff or its baseline.
function currentPlainText() {
  const source = document.createElement('div');
  source.innerHTML = latestNote.html;
  Object.assign(source.style, {
    position: 'fixed', left: '-10000px', top: '0',
    width: `${$('viewport').clientWidth}px`, pointerEvents: 'none',
  });
  source.setAttribute('aria-hidden', 'true');
  document.body.appendChild(source);
  try { return source.innerText; }
  finally { source.remove(); }
}
async function copyNote() {
  if (!latestNote || editing) return;
  try {
    if (latestNote.capture) {
      await invoke('copy_note_image');
    } else {
      await window.__TAURI__.clipboardManager.writeText(currentPlainText());
    }
    $('copy').dataset.copied = 'true';
    $('copy').title = latestNote.capture ? '이미지 복사됨' : '복사됨';
    $('copy').setAttribute('aria-label', $('copy').title);
    $('copy-status').hidden = false;
    clearTimeout(copyFeedback);
    copyFeedback = setTimeout(() => {
      $('copy-status').hidden = true;
      delete $('copy').dataset.copied;
      $('copy').title = latestNote?.capture ? '이미지 복사' : '본문 복사';
      $('copy').setAttribute('aria-label', $('copy').title);
    }, 1200);
  } catch (error) { failure(error); }
}
$('copy').addEventListener('click', copyNote);
$('content').addEventListener('click', (event) => { if (event.target.closest('a')) event.preventDefault(); });
// Native movement arrives even when WKWebView belongs to a background app.
window.__TAURI__.event.listen('note-pointer-over', ({ payload }) => {
  document.body.classList.toggle('pointer-over', payload);
}).catch(failure);
refresh();

$('clear-diff').addEventListener('click', async () => {
  if (highlightRevision === null || acknowledging) return;
  const observed = highlightRevision;
  acknowledging = true;
  $('clear-diff').disabled = true;
  try {
    const accepted = await invoke('acknowledge_feedback', { revision: observed });
    if (accepted) acknowledgedRevision = Math.max(acknowledgedRevision, observed);
    if (accepted && highlightRevision === observed) {
      highlightRevision = null;
      displayedHtml = latestNote.html;
      $('content').innerHTML = displayedHtml;
      $('clear-diff').hidden = true;
      $('diff-actions').hidden = true;
      $('error').hidden = true;
      needsFit = true;
      await fitContent();
    }
  } catch (error) { failure(error); }
  finally {
    acknowledging = false;
    $('clear-diff').disabled = false;
  }
});

// Pointer contact only updates opacity; diff acknowledgement is explicit.
document.documentElement.addEventListener('pointerenter', () => invoke('sync_opacity').catch(failure));
document.documentElement.addEventListener('pointerleave', () => invoke('sync_opacity').catch(failure));

let warpTipKey = 'floatnote.warp-tip-dismissed';
let warpTipDismissed = false;
let warpTipTimer;
function isWarpTipDismissed() {
  try { return warpTipDismissed || localStorage.getItem(warpTipKey) === 'true'; }
  catch { return warpTipDismissed; }
}
function showWarpTip() {
  if (isWarpTipDismissed()) return;
  $('warp-tip').hidden = false;
  clearTimeout(warpTipTimer);
  warpTipTimer = setTimeout(() => { $('warp-tip').hidden = true; }, 12000);
}
$('warp-tip-close').addEventListener('click', () => { $('warp-tip').hidden = true; });
$('warp-tip-dismiss').addEventListener('click', () => {
  try { localStorage.setItem(warpTipKey, 'true'); }
  catch (error) { failure(`안내 설정을 저장하지 못했습니다: ${error}`); return; }
  warpTipDismissed = true;
  $('warp-tip').hidden = true;
});
let warping = false;
$('warp').addEventListener('click', async () => {
  if (warping) return;
  warping = true;
  try { await invoke('warp_note'); showWarpTip(); }
  catch (error) { failure(error); }
  finally { warping = false; }
});
async function pollWarp() {
  try {
    if (!warping && await invoke('poll_warp')) showWarpTip();
    if (isWarpTipDismissed()) $('warp-tip').hidden = true;
  } catch (error) { failure(error); }
  setTimeout(pollWarp, 100);
}
pollWarp();


function displayFold(value) {
  value = Boolean(value);
  if (folded === value) return;
  folded = value;
  document.body.classList.toggle('folded', folded);
  $('fold-toggle').setAttribute('aria-expanded', String(!folded));
  $('fold-toggle').setAttribute('aria-label', folded ? '메모 펼치기' : '메모 접기');
  $('fold-toggle').title = `${folded ? '메모 펼치기' : '메모 접기'} · Control을 누른 채 메모에 포인터를 올리면 접기·펼치기 전환`;
  $('fold-title').hidden = !folded;
}
let changingFold = false;
async function toggleFold() {
  if (changingFold) return;
  changingFold = true;
  try { displayFold(await invoke('toggle_fold')); }
  catch (error) { failure(error); }
  finally { changingFold = false; }
}
// Capture folded clicks before the icon handler so one click toggles only once.
document.addEventListener('click', (event) => {
  if (!folded || event.button !== 0) return;
  event.preventDefault();
  event.stopPropagation();
  return toggleFold();
}, true);
$('fold-toggle').addEventListener('click', toggleFold);
async function pollFold() {
  try { displayFold(await invoke('poll_fold')); }
  catch (error) { failure(error); }
  setTimeout(pollFold, 100);
}
pollFold();
