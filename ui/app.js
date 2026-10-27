'use strict';
import { themeFor } from './theme.mjs';
import { feedbackView, durations } from './feedback-state.mjs';
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
let exporting = false;
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
  const height = Math.ceil(rect.height + 2);
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
let restoringEditorFocus = false;
function endEditing() {
  editing = false;
  document.body.classList.toggle('editing', false);
  $('editor').hidden = true;
  $('content').hidden = false;
  $('copy').disabled = false;
  $('edit').disabled = false;
  clearNotice();
  needsFit = true;
  fitContent();
  restoringEditorFocus = true;
  $('edit').focus();
  restoringEditorFocus = false;
}
function beginEditing() {
  if (!latestNote || editing) return;
  editing = true;
  document.body.classList.toggle('editing', true);
  editRevision = latestNote.revision;
  $('markdown').value = latestNote.markdown;
  $('content').hidden = true;
  $('editor').hidden = false;
  $('copy').disabled = true;
  $('edit').disabled = true;
  clearNotice();
  $('markdown').focus();
  $('markdown').setSelectionRange(0, 0);
  $('markdown').scrollTop = 0;
}
$('edit').addEventListener('click', beginEditing);

async function save() {
  if (!editing || saving) return;
  saving = true;
  publishFeedback();
  $('markdown').readOnly = true;
  try {
    latestNote = await invoke('save_note', { markdown: $('markdown').value, expectedRevision: editRevision });
    highlightRevision = null;
    acknowledgedRevision = latestNote.revision;

    // Direct saves confirm the content; subsequent external revisions can highlight again.
    $('content').innerHTML = latestNote.html;
    displayedHtml = $('content').innerHTML;
    revision = latestNote.revision;
    endEditing();
    notify('저장했습니다');
  } catch (error) { failure(error); }
  finally {
    saving = false;
    publishFeedback();
    $('markdown').readOnly = false;
  }
}
let notice = null;
let noticeTimer;
let feedbackSignature;
let publishing = false;
let memoVisible = false;
let publishAgain = false;
function clearNotice() { clearTimeout(noticeTimer); notice = null; publishFeedback(); }
function notify(text, kind = 'result', persistent = false) {
  if (notice?.kind === 'error' && kind !== 'error') return;
  clearTimeout(noticeTimer);
  notice = { text, kind };
  publishFeedback();
  if (!persistent) noticeTimer = setTimeout(clearNotice, durations[kind] || durations.result);
}
function failure(error) { notify(String(error), 'error', editing); }
async function publishFeedback() {
  if (!memoVisible) return;
  if (publishing) { publishAgain = true; return; }
  const view = feedbackView({ editing, saving, acknowledging, highlightRevision, notice, sourcePath: latestNote?.source_path, color: latestNote?.color });
  const signature = JSON.stringify(view);
  if (signature === feedbackSignature) return;
  publishing = true;
  try { await invoke('present_feedback', { view }); feedbackSignature = signature; }
  catch (error) { console.error('Feedback 창 표시 실패:', error); }
  finally { publishing = false; if (publishAgain) { publishAgain = false; publishFeedback(); } }
}
async function exportNote() {
  if (!latestNote || exporting || saving) return;
  exporting = true;
  $('export').disabled = true;
  try {
    const path = await invoke('export_note', { markdown: editing ? $('markdown').value : null });
    if (path) {
      notify(`저장했습니다: ${path}`);
    }
  } catch (error) { failure(error); }
  finally { exporting = false; $('export').disabled = false; }
}
$('export').addEventListener('click', exportNote);
async function refresh() {
  try {
    const state = await invoke('snapshot');
    latestNote = state.note;
    memoVisible = state.visible !== false;
    if (!memoVisible) feedbackSignature = undefined;
    document.body.classList.toggle('pointer-over', !!state.hovered);
    $('dev-instance').hidden = !state.dev_label;
    $('dev-instance').textContent = state.dev_label ? 'DEV' : '';
    $('dev-instance').title = state.dev_label || '';
    $('dev-instance').setAttribute('aria-label', state.dev_label || '');
    $('fold-title').textContent = state.dev_label ? `[${state.dev_label}] ${state.note.title}` : state.note.title;
    $('fold-title').title = state.note.title;
    $('edit').disabled = editing || !!state.note.capture;
    $('font-size').disabled = !!state.note.capture;
    $('copy').disabled = editing;
    $('copy').title = state.note.capture ? '이미지 복사' : '본문 복사';
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
    publishFeedback();
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
  if (folded || event.button !== 0 || event.target.closest('button, #editor, #tooltip, .resize')) return;
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
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
    event.preventDefault();
    exportNote();
  }
});
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
    notify('복사했습니다');
  } catch (error) { failure(error); }
}
$('copy').addEventListener('click', copyNote);
$('content').addEventListener('click', (event) => { if (event.target.closest('a')) event.preventDefault(); });
// Native movement arrives even when WKWebView belongs to a background app.
window.__TAURI__.event.listen('note-pointer-over', ({ payload }) => {
  document.body.classList.toggle('pointer-over', payload);
}).catch(failure);
refresh();

async function acknowledge() {
  if (highlightRevision === null || acknowledging) return;
  const observed = highlightRevision;
  acknowledging = true;
  publishFeedback();
  try {
    const accepted = await invoke('acknowledge_feedback', { revision: observed });
    if (accepted) acknowledgedRevision = Math.max(acknowledgedRevision, observed);
    if (accepted && highlightRevision === observed) {
      highlightRevision = null;
      displayedHtml = latestNote.html;
      $('content').innerHTML = displayedHtml;

      clearNotice();
      needsFit = true;
      await fitContent();
    }
  } catch (error) { failure(error); }
  finally {
    acknowledging = false;
    publishFeedback();
  }
}

// Pointer contact only updates opacity; diff acknowledgement is explicit.
document.documentElement.addEventListener('pointerenter', () => invoke('sync_opacity').catch(failure));
document.documentElement.addEventListener('pointerleave', () => invoke('sync_opacity').catch(failure));

function showWarpTip() {
  notify('반대편으로 워프 · macOS: Option + 메모에 포인터', 'tip');
}
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

window.__TAURI__.event.listen('feedback-action', ({ payload }) => {
  if (payload.owner !== win.label) return;
  const action = payload.action;
  if (action === 'acknowledge' && payload.revision !== highlightRevision) return;
  if (action === 'export') return exportNote();
  if (action === 'save') return save();
  else if (action === 'cancel' && editing && !saving) return endEditing();
  else if (action === 'acknowledge' && !editing) return acknowledge();
}).catch(failure);

// Themed Tooltip on pointer hover and keyboard focus; native title stays in data.
for (const button of document.querySelectorAll('#controls button')) {
  const tooltip = $('tooltip');
  function showTooltip() {
    if (button.title) { button.dataset.tooltip = button.title; button.removeAttribute('title'); }
    tooltip.textContent = button.dataset.tooltip || button.getAttribute('aria-label');
    tooltip.hidden = false;
    button.setAttribute('aria-describedby', 'tooltip');
    const rect = button.getBoundingClientRect();
    const bounds = tooltip.getBoundingClientRect();
    tooltip.style.left = `${Math.max(4, Math.min(rect.right - bounds.width, window.innerWidth - bounds.width - 4))}px`;
    tooltip.style.top = `${Math.max(0, Math.min(rect.bottom + 4, window.innerHeight - bounds.height))}px`;
  }
  function hideTooltip() { tooltip.hidden = true; button.removeAttribute('aria-describedby'); }
  button.addEventListener('pointerenter', showTooltip);
  button.addEventListener('focus', () => {
    if (!restoringEditorFocus && button.matches(':focus-visible')) showTooltip();
  });
  button.addEventListener('pointerleave', hideTooltip);
  button.addEventListener('blur', hideTooltip);
  button.addEventListener('click', hideTooltip);
}

window.__TAURI__.event.listen('feedback-connection', ({ payload: c }) => {
  if (c.owner !== win.label) return;
  const attached = c.edge === 'below' || c.edge === 'above';
  $('connected-outline').hidden = !attached;
  document.body.classList.toggle('feedback-below', attached && c.edge === 'below');
  document.body.classList.toggle('feedback-above', attached && c.edge === 'above');
  document.body.classList.toggle('feedback-full', attached && c.width >= c.memoWidth);
  if (!attached) return;
  const w = c.memoWidth, h = c.memoHeight, join = Math.max(0, w - c.width);
  const left = join ? `M ${join} ${h-.5} H 4 Q .5 ${h-.5} .5 ${h-4}` : `M .5 ${h}`;
  $('connected-path').setAttribute('d', `${left} V 4 Q .5 .5 4 .5 H ${w-4} Q ${w-.5} .5 ${w-.5} 4 V ${h}`);
  $('connected-path').setAttribute('transform', c.edge === 'above' ? `translate(0 ${h}) scale(1 -1)` : '');
}).catch(failure);
