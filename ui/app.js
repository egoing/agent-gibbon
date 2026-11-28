'use strict';
import { createMemoEditor } from './generated/milkdown.js';
import { memoThemeFor } from './theme.mjs';
import { readMemoAppearance } from './appearance.mjs';
import { feedbackView, durations } from './feedback-state.mjs';
import { createFeedback } from './feedback.js';
const invoke = window.jajanInvoke || window.__TAURI__.core.invoke;
const win = window.__TAURI__.window.getCurrentWindow();
const $ = (id) => document.getElementById(id);
const isMac = /Mac|iPhone|iPad/.test(navigator.platform);
const renderFeedback = createFeedback(document, (action, revision) => invoke('feedback_action', { action, revision }).catch(failure));
const shortcut = key => isMac ? `Command ${key}` : `Ctrl ${key}`;
window.jajanLanguage.bind($('export'), 'title', window.jajanLanguage.msg('{label} · {shortcut}', {label: window.jajanLanguage.msg('파일 내보내기'), shortcut: shortcut('S')}));
window.jajanLanguage.bind($('export'), 'aria-label', '파일 내보내기');
let revision = -1;
let appearanceSignature;
function applyMemoAppearance(color) {
  const theme = memoThemeFor(color, readMemoAppearance(window));
  const next = JSON.stringify(theme);
  if (next === appearanceSignature) return;
  for (const key of ['background', 'foreground', 'border', 'panel', 'error']) document.documentElement.style.setProperty(`--${key}`, theme[key]);
  document.documentElement.style.setProperty('--secondary', theme.secondary || theme.foreground);
  if (document.documentElement.dataset) document.documentElement.dataset.memoDesign = readMemoAppearance(window).design;
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
let composition = false;
let sourceProtected = false;
let editor;
let editorHeightRequested = false;
let initialInputHandled = false;
let firstFrameReady = false;
let editorReady;
function ensureEditor() { return editorReady ||= createMemoEditor($('editor'), editorChanged).then(value => { editor = value; return value; }); }
let saving = false;
let savePromise;
let autosaveTimer;
let autosaveBlocked = false;
let discardedDraft = null;
let discardGeneration = 0;
const externalDraftMessage = '메모가 외부에서 갱신되었습니다. 초안을 복사한 뒤 취소하면 최신 내용을 확인할 수 있습니다.';
function restoredConflict() {
  const origin = editor?.status().draftRevision;
  return editing && origin != null && origin !== latestNote?.revision;
}
window.jajanUpdateActivity = () => ({ editing: editing || composition, saving, exporting });
window.jajanUpdateBusy = () => editing || composition || saving || exporting;
let exporting = false;
let needsFit = true;
let fitting = false;
let layoutPending = false;
let folded = false;
async function fitContent(flow) {
  if (latestNote?.capture || !needsFit || fitting || ((editing || composition) && !layoutPending) || folded) return;
  const measuredRevision = latestNote.revision;
  const {width, height} = (flow?.measure || ((_stage, work) => work()))('measure', () => {
    // Measure natural flow independently of the current window height.
    const measure = $('viewport').cloneNode(true);
    measure.id = 'measure-viewport';
    const editorState = editor?.status();
    const editorBody = measure.querySelector('#memo-editor');
    if (editorBody && editorState?.revision === measuredRevision && !editing && !composition) {
      // Milkdown's empty paragraphs and line breaks occupy space in the editor.
      editorBody.querySelectorAll('.change-removed[contenteditable="false"]').forEach(node => node.remove());
      measure.replaceChildren(editorBody);
    } else {
      // A pending open can need the saved revision while a local draft is active.
      measure.querySelector('#editor')?.remove();
      const measuredContent = measure.querySelector('#content');
      measuredContent.hidden = false;
      measuredContent.innerHTML = latestNote.html;
    }
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
  fitting = true;
  // Consume this request before awaiting IPC so a newer request stays pending.
  needsFit = false;
  try {
    const accepted = await (flow?.invoke || invoke)('fit_note', { width, height, revision: measuredRevision });
    if (!accepted) needsFit = true;
  } catch (error) {
    needsFit = true;
    failure(error);
  } finally { fitting = false; }
}
function contentAction() { return invoke('content_action').catch(failure); }

for (const id of ['toolbar-previous', 'toolbar-next']) $(id).addEventListener('click', contentAction);
document.addEventListener('wheel', contentAction, { passive: true });
window.jajanContentSize.bind('text', $('font-size'), size => {
  contentAction();
  document.documentElement.style.setProperty('--font-size', `${size}px`);
  needsFit = true;
  fitContent();
}, failure);
function editorChanged(state) {
  const wasDirty = editing;
  editing = state.dirty;
  composition = state.composing;
  sourceProtected = state.protected;
  document.body.classList.toggle('dirty', editing);
  $('save').disabled = !editing || saving || composition || sourceProtected;
  if (!editing && wasDirty) {
    displayedHtml = undefined;
    if (state.draftRevision != null) { autosaveBlocked = false; clearNotice(); }
  }
  if (editing && !wasDirty) {
    discardedDraft = null;
    readRevision = null;
    clearNotice();
    if (!editorHeightRequested && memoVisible && !folded) {
      editorHeightRequested = true;
      invoke('enter_note_editor', { revision: latestNote.revision }).catch(failure);
    }
  }
  if (restoredConflict()) notify(externalDraftMessage, 'error', true);
  contentAction();
  scheduleAutosave();
  publishFeedback();
}
function scheduleAutosave() {
  clearTimeout(autosaveTimer);
  if (!editing || composition || saving || sourceProtected || autosaveBlocked || restoredConflict()) return;
  autosaveTimer = setTimeout(function autosaveTick() { return save(true); }, 600);
}
function endEditing() {
  if (!editing || !editor || latestNote?.capture || saving || exporting || sourceProtected || composition || editor.status().composing) return;
  clearTimeout(autosaveTimer);
  autosaveBlocked = false;
  discardedDraft = editor.discard(latestNote);
  discardGeneration++;
  editing = false;
  document.body.classList.toggle('dirty', false);
  clearNotice();
  editorChanged(editor.status());
  displayedHtml = undefined;
  refresh(false);
}

function restoreDiscarded(generation) {
  if (!discardedDraft || generation !== discardGeneration || !memoVisible || folded || stashActive || latestNote?.capture ||
      editing || saving || exporting || composition || editor.status().composing) return;
  // A snapshot received during composition may not have reached the editor yet.
  const state = editor.sync(latestNote);
  if (state.protected || state.dirty || state.conflict) return;
  if (editor.restore(discardedDraft)) {
    discardedDraft = null;
    if (!editing) clearNotice();
    // The IPC response may arrive after the user has moved to another control/app.
    if (document.hasFocus() && (document.activeElement === document.body ||
        document.activeElement?.dataset?.action === 'restore_draft')) editor.focus();
    publishFeedback();
  }
}

function save(automatic = false) {
  clearTimeout(autosaveTimer);
  if (saving) return savePromise;
  if (!editing) return Promise.resolve(true);
  if (exporting || composition || sourceProtected || editor?.status().composing || automatic && (autosaveBlocked || restoredConflict())) return Promise.resolve(false);
  savePromise = performSave();
  return savePromise;
}
async function performSave() {
  saving = true;
  editor.pending(true);
  publishFeedback();
  $('save').disabled = true;
  try {
    const snapshot = editor.capture();
    await (window.jajanLocalFlow || ((_op, work) => work({invoke, stage:()=>{}})))('note.save', async flow => {
      const savedNote = await flow.invoke('save_note', { markdown: snapshot.markdown, expectedRevision: snapshot.revision });
      if (!latestNote || savedNote.revision >= latestNote.revision) latestNote = savedNote;
      editor.saved(savedNote, snapshot);
      autosaveBlocked = false;
      clearNotice();
      editing = editor.status().dirty;
      document.body.classList.toggle('dirty', editing);
      highlightRevision = null;
      acknowledgedRevision = savedNote.revision;
      flow.stage('response');
      displayedHtml = undefined;
      revision = Math.max(revision, savedNote.revision);
      needsFit = true;
      if (!editing) notify('저장됨');
      await fitContent(flow).catch(failure);
      flow.frame?.();
    });
    return true;
  } catch (error) {
    autosaveBlocked = true;
    failure(error);
    return false;
  } finally {
    editor.pending(false);
    saving = false;
    $('save').disabled = !editing || composition || sourceProtected;
    scheduleAutosave();
    publishFeedback();
  }
}
async function flushAnd(action) {
  // Hiding keeps the editor alive, but close can destroy it: flush before both.
  if (composition || editor?.status().composing) return;
  if (saving && !await savePromise) return;
  if (editing && (!await save() || editing)) return;
  return invoke(action).catch(failure);
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
  const view = feedbackView({ editing, saving, exporting, composition, sourceProtected, autosaveBlocked: autosaveBlocked || restoredConflict(), recovery: discardedDraft ? discardGeneration : null, unified: !latestNote?.capture, acknowledging, highlightRevision, notice, color: latestNote?.color });
  renderFeedback(view, folded || !memoVisible || stashActive);
  if (!memoVisible) return;
  const signature = JSON.stringify({ folded, stashActive, view });
  if (signature === feedbackSignature) return;
  publishing = true;
  try { await invoke('present_feedback', { view: {...view, message: window.jajanLanguage.source(view.message)} }); feedbackSignature = signature; }
  catch (error) { console.error('Feedback 상태 기록 실패:', error); }
  finally { publishing = false; if (publishAgain) { publishAgain = false; publishFeedback(); } }
}
async function exportNote() {
  if (!latestNote || editing || exporting || saving) return;
  contentAction();
  exporting = true;
  publishFeedback();
  $('export').disabled = true;
  try {
    const path = await invoke('export_note', { markdown: null });
    if (path) {
      notify(window.jajanLanguage.msg('파일로 내보냈습니다: {path}', {path}));
    }
  } catch (error) { failure(error); }
  finally { exporting = false; $('export').disabled = false; publishFeedback(); }
}
$('export').addEventListener('click', exportNote);
async function refresh(schedule = true) {
  let region = false;
  let firstFrameColor = '#20252e';
  try {
    await (window.jajanLocalFlow || ((_op, work) => work({invoke, stage:()=>{}})))('note.refresh', async flow => {
      region = !firstFrameReady && await window.jajanRegionFrame?.placement;
      const state = await flow.invoke('snapshot');
      flow.stage('response');
      firstFrameColor = state.note.color;
      if (!state.note.capture) await ensureEditor();
      if (state.note.revision < revision) return;
      latestNote = state.note;
      layoutPending = !!state.layout_pending;
      if (layoutPending) needsFit = true;
      memoVisible = state.visible !== false;
      stashActive = !!state.stash?.active;
      if (!memoVisible) feedbackSignature = undefined;
      document.body.classList.toggle('pointer-over', !!state.hovered);
      $('fold-title').textContent = state.note.title;
      $('fold-title').title = state.note.title;
      $('save').hidden = true;
      $('export').hidden = !state.note.capture;
      $('editor').hidden = !!state.note.capture;
      $('content').hidden = !state.note.capture;
      if (!state.note.capture) {
        const editorState = editor.sync(state.note);
        composition = editorState.composing;
        sourceProtected = editorState.protected;
        $('save').disabled = !editing || saving || composition || sourceProtected;
        if (editorState.conflict && editing && notice?.text !== externalDraftMessage) notify(externalDraftMessage, 'error', true);
        else if (sourceProtected && !notice) notify('지원하지 않는 문법의 손실을 막기 위해 원문을 보호하고 있습니다. 원문 복사를 사용할 수 있습니다.', 'error', true);
      }
      window.jajanMemoToolbar.setKind(state.note.capture ? 'image' : 'text');
      $('font-size').disabled = !!state.note.capture;
      $('copy').disabled = false;
      const copyLabel = state.note.capture ? '이미지 복사' : sourceProtected ? '원문 전체 복사' : '본문 전체 복사';
      window.jajanLanguage.bind($('copy'), 'title', state.note.capture ? window.jajanLanguage.msg('{label} · {shortcut}', {label: window.jajanLanguage.msg(copyLabel), shortcut: shortcut('C')}) : copyLabel);
      window.jajanLanguage.bind($('copy'), 'aria-label', copyLabel);
      document.body.classList.toggle('image-note', !!state.note.capture);
      document.body.classList.toggle('capture-toolbar-below', !!state.capture_toolbar_below);

      document.body.classList.toggle('text-note', !state.note.capture);
      // Refresh CSS independently of revisions; preserve drafts and local trace stages.
      applyMemoAppearance(state.note.color);
      if (revision !== state.note.revision) {
        if (state.note.title) window.jajanLanguage.raw($('viewport'), 'aria-label', state.note.title);
        else window.jajanLanguage.bind($('viewport'), 'aria-label', '메모 내용');
        // HTML is generated and sanitized in Rust; no user HTML reaches this sink.
        // Feedback rendering is synchronized below even when revision is unchanged.
        revision = state.note.revision;
        needsFit = true;
      }
      highlightRevision = (state.highlight_revision ?? -1) > acknowledgedRevision ? state.highlight_revision : null;
      publishFeedback();
      const html = highlightRevision !== null ? state.note.change_html : (state.note.completion_html ?? state.note.html);
      const changed = displayedHtml !== html;
      if (changed && state.note.capture) {
        (flow.measure || ((_stage, work) => work()))('dom', () => { $('content').innerHTML = html; });
        if (state.note.capture) {
          const img = document.createElement('img');
          img.alt = '화면 캡처'; img.draggable = false;
          img.src = await invoke('note_image');
          if (region) await img.decode();
          $('content').replaceChildren(img);
        }
        displayedHtml = html;
      } else if (changed && !editing && !composition) {
        editor.highlight(html);
        displayedHtml = html;
      }
      if (region) await window.jajanRegionFrame.ready();
      firstFrameReady = true;
      if (!initialInputHandled && state.note.edit_on_open && !state.note.capture && memoVisible && !state.folded && !folded) {
        initialInputHandled = true;
        editor.focus();
      }
      $('pin').setAttribute('aria-pressed', String(state.always_on_top));
      await fitContent(flow);
      const reading = memoVisible && (state.focused || state.feedback_surface?.focused);
      if (reading && !editing && !composition && !sourceProtected) readRevision = state.note.revision;
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
  } catch (error) {
    if (region) {
      // The snapshot, editor or image can fail before a hidden window has any
      // visible feedback. Present a themed error and let normal refresh retry.
      applyMemoAppearance(firstFrameColor);
      renderFeedback({message: String(error), error: true, buttons: []});
      await window.jajanRegionFrame.ready().catch(failure);
    } else failure(error);
  }
  if (schedule) setTimeout(refresh, 350);
}
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
if (isMac && typeof ResizeObserver !== 'undefined') {
  const close = $('close');
  let previous = '';
  const syncCloseRect = () => {
    const { x, y, width, height } = close.getBoundingClientRect();
    const rect = [x, y, width, height];
    const signature = rect.join(',');
    if (signature === previous) return;
    previous = signature;
    invoke('close_hit_rect', { rect }).catch(() => { previous = ''; });
  };
  const resize = new ResizeObserver(syncCloseRect);
  for (const element of [document.documentElement, $('controls'), close]) resize.observe(element);
  new MutationObserver(syncCloseRect).observe(document.body, { attributes: true, attributeFilter: ['class'] });
  syncCloseRect();
}
$('close').addEventListener('click', () => flushAnd('close_note'));
function bodyEditorHasFocus() {
  return !latestNote?.capture && document.activeElement?.id === 'memo-editor';
}
document.addEventListener('keydown', (event) => {
  if (event.isComposing || event.keyCode === 229 || composition || editor?.status().composing) return;
  if (event.key === 'Escape') {
    event.preventDefault();
    flushAnd('hide_note');
  }
  const copyModifier = isMac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
  if (latestNote && !bodyEditorHasFocus() && copyModifier && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 'c') {
    event.preventDefault();
    copyNote();
  }
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
    event.preventDefault();
    if (!event.isComposing && !event.repeat && !event.altKey && !event.shiftKey && event.keyCode !== 229) saveShortcut();
  }
});
function saveShortcut() { return latestNote?.capture ? exportNote() : save(); }
window.jajanLanguage.bind($('save'), 'title', window.jajanLanguage.msg('{label} · {shortcut}', {label: window.jajanLanguage.msg('저장'), shortcut: shortcut('S')}));
window.jajanLanguage.bind($('save'), 'aria-label', window.jajanLanguage.msg('{label} · {shortcut}', {label: window.jajanLanguage.msg('저장'), shortcut: shortcut('S')}));
$('save').addEventListener('click', () => save());
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
// WKWebView can receive the native Edit > Copy command without a keydown.
document.addEventListener('copy', event => {
  if (!latestNote || !event.isTrusted || bodyEditorHasFocus()) return;
  event.preventDefault();
  event.stopPropagation();
  copyNote();
}, true);
async function copyNote() {
  if (!latestNote) return;
  contentAction();
  try {
    if (latestNote.capture) {
      await invoke('copy_note_image');
    } else {
      const text = sourceProtected ? latestNote.markdown : editing ? $('memo-editor').innerText : currentPlainText();
      await window.__TAURI__.clipboardManager.writeText(text);
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
  if (acknowledging || editing || composition || sourceProtected || latestNote?.revision !== observed) return;
  acknowledging = true;
  publishFeedback();
  try {
    const accepted = await invoke('acknowledge_feedback', { revision: observed });
    if (accepted) acknowledgedRevision = Math.max(acknowledgedRevision, observed);
    if (accepted && latestNote.revision === observed) {
      latestNote.completion_html = null;
      highlightRevision = null;
      displayedHtml = latestNote.html;
      if (!latestNote.capture) editor.highlight(displayedHtml);
      else $('content').innerHTML = displayedHtml;

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

async function pollWarp() {
  try {
    await invoke('poll_warp');
  } catch (error) { failure(error); }
  setTimeout(pollWarp, 100);
}
pollWarp();


function displayFold(value) {
  value = Boolean(value);
  if (folded === value) return;
  folded = value;
  feedbackSignature = undefined;
  window.jajanToolbarFold?.(folded);
  document.body.classList.toggle('folded', folded);
  $('fold-toggle').setAttribute('aria-expanded', String(!folded));
  window.jajanLanguage.bind($('fold-toggle'), 'aria-label', folded ? '메모 펼치기' : '메모 접기');
  window.jajanLanguage.bind($('fold-toggle'), 'title', folded ? '메모 펼치기' : '메모 접기');
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
window.jajanToggleNote = toggleFold;
window.jajanApplyFold = displayFold;
async function pollFold() {
  try { displayFold(await invoke('poll_fold')); }
  catch (error) { failure(error); }
  setTimeout(pollFold, 100);
}
pollFold();

window.__TAURI__.event.listen('feedback-action', ({ payload }) => {
  if (payload.owner !== win.label) return;
  const action = payload.action;
  if (action === 'restore_draft') return restoreDiscarded(payload.revision);
  if (action === 'save-shortcut') return saveShortcut();
  if (action === 'export') return exportNote();
  if (action === 'save') return save();
  else if (action === 'cancel' && editing && !saving && !exporting) return endEditing();
}).catch(failure);
