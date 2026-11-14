'use strict';
import { memoThemeFor } from './theme.mjs';
import { readMemoAppearance } from './appearance.mjs';
import { feedbackView, durations } from './feedback-state.mjs';
import { createFeedback } from './feedback.js';
const invoke = window.gibbonInvoke || window.__TAURI__.core.invoke;
const win = window.__TAURI__.window.getCurrentWindow();
const $ = (id) => document.getElementById(id);
const isMac = /Mac|iPhone|iPad/.test(navigator.platform);
const renderFeedback = createFeedback(document, (action, revision) => invoke('feedback_action', { action, revision }).catch(failure));
const shortcut = key => isMac ? `Command ${key}` : `Ctrl ${key}`;
$('export').title = `파일 내보내기 · ${shortcut('S')}`;
$('export').setAttribute('aria-label', '파일 내보내기');
let revision = -1;
let appearanceSignature;
function applyMemoAppearance(color) {
  const theme = memoThemeFor(color, readMemoAppearance(window));
  const next = JSON.stringify(theme);
  if (next === appearanceSignature) return;
  for (const key of ['background', 'foreground', 'border', 'panel', 'error']) document.documentElement.style.setProperty(`--${key}`, theme[key]);
  document.documentElement.style.colorScheme = theme.scheme;
  appearanceSignature = next;
}
let latestNote;
let displayedHtml;
let highlightRevision = null;
let acknowledging = false;
let acknowledgedRevision = -1;
let readRevision = null;
let wasReading = false;
let editing = false;
let initialEditHandled = false;
let saving = false;
window.gibbonUpdateActivity = () => ({ editing, saving, exporting });
window.gibbonUpdateBusy = () => editing || saving || exporting;
let exporting = false;
let editRevision;
let needsFit = true;
let fitting = false;
let layoutPending = false;
let folded = false;
async function fitContent(flow) {
  if (latestNote?.capture || !needsFit || fitting || (editing && !layoutPending) || folded) return;
  fitting = true;
  const {width, height} = (flow?.measure || ((_stage, work) => work()))('measure', () => {
    // Measure natural flow independently of the current window height.
    const measure = $('viewport').cloneNode(true);
    measure.id = 'measure-viewport';
    measure.querySelector('#editor')?.remove();
    measure.querySelector('#controls')?.remove();
    const measuredContent = measure.querySelector('#content');
    measuredContent.hidden = false;
    measuredContent.innerHTML = latestNote.html;
    Object.assign(measure.style, {
      position: 'fixed', left: '-10000px', top: '0', visibility: 'hidden',
      display: 'flow-root', flex: 'none', width: '298px', minWidth: '298px',
      maxWidth: '298px', height: 'auto', minHeight: '0', maxHeight: 'none',
      overflow: 'visible', padding: '0 3px 3px', overflowWrap: 'anywhere', pointerEvents: 'none',
    });
    document.body.appendChild(measure);
    const rect = measure.getBoundingClientRect();
    const width = Math.ceil(rect.width + 2);
    const height = Math.ceil(rect.height + 2 + $('controls').getBoundingClientRect().height);
    measure.remove();
    return {width, height};
  });
  flow?.stage('fit');
  try { needsFit = !await (flow?.invoke || invoke)('fit_note', { width, height, revision: latestNote.revision }); }
  catch (error) { failure(error); }
  finally { fitting = false; }
}
function contentAction() { return invoke('content_action').catch(failure); }
$('markdown').addEventListener('input', contentAction);
for (const id of ['toolbar-previous', 'toolbar-next']) $(id).addEventListener('click', contentAction);
document.addEventListener('wheel', contentAction, { passive: true });
const fontSizes = [12, 14, 16, 18];
let fontIndex = 0;
$('font-size').addEventListener('click', () => {
  contentAction();
  fontIndex = (fontIndex + 1) % fontSizes.length;
  const size = fontSizes[fontIndex];
  document.documentElement.style.setProperty('--font-size', `${size}px`);
  needsFit = true;
  fitContent();
  $('font-size').title = `글자 크기 ${size}px · 클릭하여 변경`;
  $('font-size').setAttribute('aria-label', `글자 크기 ${size}px, 클릭하여 변경`);
});
let restoringEditorFocus = false;
function endEditing(flow) {
  contentAction();
  editing = false;
  document.body.classList.toggle('editing', false);
  $('editor').hidden = true;
  $('content').hidden = false;
  $('copy').disabled = false;
  $('edit').disabled = false;
  clearNotice();
  needsFit = true;
  const fitted = fitContent(flow);
  flow?.follow?.(fitted);
  restoringEditorFocus = true;
  $('edit').focus();
  restoringEditorFocus = false;
  return fitted;
}
function beginEditing() {
  if (!latestNote || latestNote.capture || editing || folded) return;
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
  // One request per entry, never from refresh, typing, or a resize observer.
  invoke('enter_note_editor', { revision: editRevision }).catch(failure);
}
$('edit').addEventListener('click', () => { contentAction(); beginEditing(); });

async function save() {
  if (!editing || saving || exporting) return;
  saving = true;
  publishFeedback();
  $('markdown').readOnly = true;
  try {
    await (window.gibbonLocalFlow || ((_op, work) => work({invoke, stage:()=>{}})))('note.save', async flow => {
      latestNote = await flow.invoke('save_note', { markdown: $('markdown').value, expectedRevision: editRevision });
      highlightRevision = null;
      acknowledgedRevision = latestNote.revision;

      flow.stage('response');
      // Direct saves confirm the content; subsequent external revisions can highlight again.
      (flow.measure || ((_stage, work) => work()))('dom', () => { $('content').innerHTML = latestNote.html; });
      displayedHtml = $('content').innerHTML;
      revision = latestNote.revision;
      endEditing(flow);
      notify('메모에 반영했습니다');
      flow.frame?.();
    });
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
let stashActive = false;
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
  if (publishing) { publishAgain = true; return; }
  const view = feedbackView({ editing, saving, exporting, acknowledging, highlightRevision, notice, sourcePath: latestNote?.source_path, color: latestNote?.color });
  renderFeedback(view, folded || !memoVisible || stashActive);
  if (!memoVisible) return;
  const signature = JSON.stringify({ folded, stashActive, view });
  if (signature === feedbackSignature) return;
  publishing = true;
  try { await invoke('present_feedback', { view }); feedbackSignature = signature; }
  catch (error) { console.error('Feedback 상태 기록 실패:', error); }
  finally { publishing = false; if (publishAgain) { publishAgain = false; publishFeedback(); } }
}
async function exportNote() {
  if (!latestNote || exporting || saving) return;
  contentAction();
  exporting = true;
  publishFeedback();
  $('export').disabled = true;
  try {
    const path = await invoke('export_note', { markdown: editing ? $('markdown').value : null });
    if (path) {
      notify(`파일로 내보냈습니다: ${path}`);
    }
  } catch (error) { failure(error); }
  finally { exporting = false; $('export').disabled = false; publishFeedback(); }
}
$('export').addEventListener('click', exportNote);
async function refresh(schedule = true) {
  try {
    await (window.gibbonLocalFlow || ((_op, work) => work({invoke, stage:()=>{}})))('note.refresh', async flow => {
      const state = await flow.invoke('snapshot');
      flow.stage('response');
      latestNote = state.note;
      layoutPending = !!state.layout_pending;
      if (layoutPending) needsFit = true;
      memoVisible = state.visible !== false;
      stashActive = !!state.stash?.active;
      if (!memoVisible) feedbackSignature = undefined;
      document.body.classList.toggle('pointer-over', !!state.hovered);
      $('fold-title').textContent = state.note.title;
      $('fold-title').title = state.note.title;
      $('edit').disabled = editing || !!state.note.capture;
      $('font-size').disabled = !!state.note.capture;
      $('copy').disabled = editing;
      const copyLabel = state.note.capture ? '이미지 복사' : '본문 복사';
      $('copy').title = `${copyLabel} · ${shortcut('C')}`;
      $('copy').setAttribute('aria-label', copyLabel);
      document.body.classList.toggle('image-note', !!state.note.capture);
      // Refresh CSS independently of revisions; preserve drafts and local trace stages.
      applyMemoAppearance(state.note.color);
      if (revision !== state.note.revision) {
        $('viewport').setAttribute('aria-label', state.note.title || '메모 내용');
        // HTML is generated and sanitized in Rust; no user HTML reaches this sink.
        // Feedback rendering is synchronized below even when revision is unchanged.
        revision = state.note.revision;
        needsFit = true;
      }
      highlightRevision = (state.highlight_revision ?? -1) > acknowledgedRevision ? state.highlight_revision : null;
      publishFeedback();
      const html = highlightRevision !== null ? state.note.change_html : (state.note.completion_html ?? state.note.html);
      const changed = displayedHtml !== html;
      if (changed) {
        (flow.measure || ((_stage, work) => work()))('dom', () => { $('content').innerHTML = html; });
        if (state.note.capture) {
          const img = document.createElement('img');
          img.alt = '화면 캡처'; img.draggable = false;
          img.src = await invoke('note_image');
          $('content').replaceChildren(img);
        }
        displayedHtml = html;
      }
      $('pin').setAttribute('aria-pressed', String(state.always_on_top));
      $('warp').setAttribute('data-direction', state.warp_right ? 'right' : 'left');
      // Region creation can first snapshot while still hidden. Defer the actual
      // editor entry until shown, rather than losing its one height request.
      if (!initialEditHandled && ((memoVisible && !state.folded) || !state.note.edit_on_open || state.note.capture)) {
        initialEditHandled = true;
        if (state.note.edit_on_open && !state.note.capture) beginEditing();
      }
      await fitContent(flow);
      const reading = memoVisible && (state.focused || state.feedback_surface?.focused);
      if (reading && !editing) readRevision = state.note.revision;
      if (wasReading && !reading) {
        const observed = readRevision;
        readRevision = null;
        if (observed !== null) await acknowledge(observed);
      }
      wasReading = !!reading;

      if (changed && document.visibilityState === 'visible') {
        // Observation only: do not delay refresh/fit waiting for a hidden WebView.
        flow.frame?.();
      }
    });
  } catch (error) { failure(error); }
  if (schedule) setTimeout(refresh, 350);
}
document.addEventListener('mousedown', (event) => {
  if (event.button !== 0 || event.target.closest('button, #editor, #tooltip, .resize')) return;
  if (folded && !event.target.closest('#controls')) return;
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
$('close').addEventListener('click', () => invoke('close_note').catch(failure));
document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape') {
    event.preventDefault();
    if (editing) { if (!saving && !exporting) endEditing(); }
    else invoke('hide_note').catch(failure);
  }
  if (!editing && latestNote && (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'c') {
    event.preventDefault();
    copyNote();
  }
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
    event.preventDefault();
    if (!event.isComposing && !event.repeat && !event.altKey && !event.shiftKey && event.keyCode !== 229) saveShortcut();
  }
});
function saveShortcut() { return editing ? save() : exportNote(); }
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
  contentAction();
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

}, { target: { kind: 'Window', label: win.label } }).catch(failure);
window.__TAURI__.event.listen('layout-measure', () => { needsFit = true; refresh(false); }).catch(failure);
refresh();

async function acknowledge(observed) {
  if (acknowledging || editing || latestNote?.revision !== observed) return;
  acknowledging = true;
  publishFeedback();
  try {
    const accepted = await invoke('acknowledge_feedback', { revision: observed });
    if (accepted) acknowledgedRevision = Math.max(acknowledgedRevision, observed);
    if (accepted && latestNote.revision === observed) {
      latestNote.completion_html = null;
      highlightRevision = null;
      displayedHtml = latestNote.html;
      $('content').innerHTML = displayedHtml;

      needsFit = true;
      await fitContent();
    }
  } catch (error) { failure(error); }
  finally {
    acknowledging = false;
    publishFeedback();
  }
}

// Pointer contact affects chrome/opacity only. Native focus is sampled together
// with the companion surface so internal control changes do not mark content read.
document.documentElement.addEventListener('pointerenter', () => invoke('sync_opacity').catch(failure));
document.documentElement.addEventListener('pointerleave', () => invoke('sync_opacity').catch(failure));
for (const event of ['tauri://focus', 'tauri://blur']) {
  window.__TAURI__.event.listen(event, () => refresh(false), { target: { kind: 'Window', label: win.label } }).catch(failure);
}

function showWarpTip() {
  if (isMac) notify('Option만 누른 채 메모에 포인터를 올려도 이동할 수 있습니다', 'tip');
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
    if (!warping) await invoke('poll_warp');
  } catch (error) { failure(error); }
  setTimeout(pollWarp, 100);
}
pollWarp();


function displayFold(value) {
  value = Boolean(value);
  if (folded === value) return;
  folded = value;
  feedbackSignature = undefined;
  window.gibbonToolbarFold?.(folded);
  document.body.classList.toggle('folded', folded);
  $('fold-toggle').setAttribute('aria-expanded', String(!folded));
  $('fold-toggle').setAttribute('aria-label', folded ? '메모 펼치기' : '메모 접기');
  $('fold-toggle').title = folded ? '메모 펼치기' : '메모 접기';
  publishFeedback();
  $('fold-title').hidden = !folded;
}
let changingFold = false;
async function toggleFold() {
  if (changingFold) return;
  changingFold = true;
  try {
    displayFold(await invoke('toggle_fold'));
    if (isMac && !folded) notify('Control만 누른 채 메모에 포인터를 올려도 접거나 펼칠 수 있습니다', 'tip');
  }
  catch (error) { failure(error); }
  finally { changingFold = false; }
}
// Capture folded clicks before the icon handler so one click toggles only once.
document.addEventListener('click', (event) => {
  if (!folded || event.button !== 0 || event.target?.closest?.('#controls')) return;
  event.preventDefault();
  event.stopPropagation();
  return toggleFold();
}, true);
$('fold-toggle').addEventListener('click', toggleFold);
window.gibbonToggleNote = toggleFold;
async function pollFold() {
  try { displayFold(await invoke('poll_fold')); }
  catch (error) { failure(error); }
  setTimeout(pollFold, 100);
}
pollFold();

window.__TAURI__.event.listen('feedback-action', ({ payload }) => {
  if (payload.owner !== win.label) return;
  const action = payload.action;
  if (action === 'save-shortcut') return saveShortcut();
  if (action === 'export') return exportNote();
  if (action === 'save') return save();
  else if (action === 'cancel' && editing && !saving && !exporting) return endEditing();
}).catch(failure);
