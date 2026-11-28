'use strict';
const invoke = window.jajanInvoke || window.__TAURI__.core.invoke;
const el = id => document.getElementById(id);
let notes = new Map(), selected = null, displayed, busy = false, detailVersion = 0;
let query = '', composing = false, retryFull = false;
let listFailed = false, imageFailed = false, cachedImage = null, captureView = null;
let unfilteredScroll = 0;
const buttons = new Map();
const search = el('memo-search');
function updateError() { el('error').hidden = !listFailed && !imageFailed; }
let visibilityView = null, visibilityVersion = 0;
const visibilityPending = new Set();
function syncVisibility() {
  if (!visibilityView) return;
  const note = notes.get(visibilityView.key);
  if (!note) return;
  const label = note.visible ? '화면에서 숨기기' : '화면에 띄우기';
  if (visibilityView.button.textContent !== label) window.jajanLanguage.bind(visibilityView.button, 'textContent', label);
  const pending = visibilityPending.has(note.key);
  if (visibilityView.button.disabled !== pending) visibilityView.button.disabled = pending;
}
// Register settings destinations here; navigation preserves each panel's DOM.
const settingsSections = ['privacy', 'skills', 'language', 'appearance', ...(el('settings-updater') ? ['updater'] : []), ...(el('settings-autostart') ? ['autostart'] : []), ...(el('settings-developer') ? ['developer'] : [])];
let selectedSettingsSection = 'privacy';
function settingsSection(selectedSection) {
  selectedSettingsSection = selectedSection;
  for (const section of settingsSections) {
    const active = section === selectedSection;
    el(`settings-${section}`).hidden = !active;
    el(`settings-${section}-nav`).setAttribute('aria-current', active ? 'page' : 'false');
  }
  if (selectedSection === 'updater') window.jajanUpdater?.refresh();
  if (selectedSection === 'developer') window.jajanDeveloperUpdater?.refresh();
  if (selectedSection === 'autostart') window.jajanAutostart?.refresh();
}
for (const section of settingsSections) {
  el(`settings-${section}-nav`).onclick = () => {
    settingsSection(section);
    if (section === 'skills') window.jajanSkills?.refresh();
  };
}
function page(settings) {
  if(window.jajanPrivacy && !window.jajanPrivacy.accepted()){window.jajanPrivacy.showWelcome();return;}
  if (settings) window.jajanDeveloperUpdater?.refresh();
  el('notes-page').hidden = settings;
  el('settings-page').hidden = !settings;
  el('notes-nav').setAttribute('aria-pressed', String(!settings));
  el('settings-nav').setAttribute('aria-pressed', String(settings));
  if (!settings) refresh();
  else if (selectedSettingsSection === 'skills') window.jajanSkills?.refresh();
  else if (selectedSettingsSection === 'updater') window.jajanUpdater?.refresh();
  else if (selectedSettingsSection === 'developer') window.jajanDeveloperUpdater?.refresh();
  else if (selectedSettingsSection === 'autostart') window.jajanAutostart?.refresh();
  else window.jajanPrivacy?.refresh().catch(()=>{});
}
// The tray's More action always opens the full list. In-window navigation keeps its filter.
window.jajanShowNotes = () => {
  search.value = '';
  composing = false;
  applyQuery();
  page(false);
};
el('notes-nav').onclick = () => page(false);
el('settings-nav').onclick = () => page(true);
el('retry').onclick = () => refresh(true);
el('refresh').onclick = () => refresh(true);
async function detail(note) {
  const version = ++detailVersion;
  imageFailed = false; updateError();
  const target = el('detail'); target.replaceChildren();
  visibilityView = null;
  captureView = null;
  if (!note) { window.jajanLanguage.bind(target, 'textContent', notes.size ? '메모를 선택하세요.' : '아직 메모가 없습니다.'); return; }
  const tools = document.createElement('div'); tools.className = 'detail-tools';
  tools.setAttribute('role', 'group'); window.jajanLanguage.bind(tools, 'aria-label', '메모 공통 도구');
  const open = document.createElement('button');
  const error = document.createElement('p'); error.className = 'detail-error'; error.hidden = true; error.setAttribute('role', 'alert');
  open.onclick = async () => {
    if (version !== detailVersion || !notes.has(note.key) || visibilityPending.has(note.key)) return;
    const desired = !notes.get(note.key)?.visible;
    visibilityPending.add(note.key); visibilityVersion++; syncVisibility(); error.hidden = true;
    try {
      const args = {id:note.id, visible:desired, kind:note.kind || 'note'};
      if (note.kind === 'web') args.session = note.session;
      const visible = await invoke('manager_set_visibility', args);
      const current = notes.get(note.key);
      if (current) current.visible = visible;
    }
    catch { if (version === detailVersion) { window.jajanLanguage.bind(error, 'textContent', '메모 표시 상태를 변경하지 못했습니다. 다시 시도하세요.'); error.hidden = false; } }
    finally { visibilityPending.delete(note.key); visibilityVersion++; syncVisibility(); }
  };
  visibilityView = { key: note.key, button: open }; syncVisibility();
  tools.append(open); tools.append(error); target.append(tools);
  if (note.kind || note.capture) {
    const title = document.createElement('h1'); title.textContent = note.title || note.id; target.append(title);
    if (note.capture) captureView = {title, image:null, request:0};
  }
  if (note.kind) {
    const kind=document.createElement('p');window.jajanLanguage.bind(kind, 'textContent', note.kind === 'web' ? '웹' : '터미널');target.append(kind);
    if(note.kind==='terminal'){const remove=document.createElement('button');window.jajanLanguage.bind(remove, 'textContent', '목록에서 삭제');remove.onclick=async()=>{try{await invoke('manager_terminal',{id:note.id,action:'delete'});await refresh(true);}catch(e){el('error').hidden=false;remove.title=String(e);}};target.append(remove);}
  } else if (note.capture) {
    await loadImage(note, captureView, false);
  } else {
    // HTML is rendered and sanitized by the same Rust pipeline as floating memos.
    const content = document.createElement('div'); content.innerHTML = note.html; target.append(content);
  }
}
async function loadImage(note, view, verifyFile) {
  const request = ++view.request;
  const isCurrent = () => captureView === view && view.request === request;
  try {
    const src = !verifyFile && cachedImage && sameImage(cachedImage.note, note)
      ? cachedImage.src : await invoke('manager_image', {id:note.id});
    if (!isCurrent()) return;
    if (view.image?.src !== src) {
      // Keep the old image (and its scrollable height) until the replacement
      // can paint. No deferred scrollTop assignment can undo a user's scroll.
      const replacement = document.createElement('img');
      replacement.src = src;
      await replacement.decode();
      if (!isCurrent()) return;
      replacement.alt = view.title.textContent;
      if (view.image) view.image.replaceWith(replacement);
      else el('detail').append(replacement);
      view.image = replacement;
    }
    cachedImage = {note, src};
    imageFailed = false;
  } catch {
    if (!isCurrent()) return;
    cachedImage = null;
    imageFailed = true;
  }
  updateError();
}
function normalize(text) {
  return (text || '').normalize('NFC').toLowerCase().replace(/\s+/g, ' ').trim();
}
function bodyText(html) {
  if (!html) return '';
  // Template contents belong to an inert document. Never attach these nodes.
  const template = document.createElement('template');
  template.innerHTML = html;
  for (const node of template.content.querySelectorAll('script,style,template,iframe,[hidden]')) node.remove();
  for (const node of template.content.querySelectorAll('p,h1,h2,h3,h4,h5,h6,br,div,li,pre,td,th,blockquote,hr')) {
    node.before(' '); node.after(' ');
  }
  return normalize(template.content.textContent);
}
function readRows(rows) {
  const next = new Map();
  for (const row of rows) {
    if (row.kind && row.kind !== 'terminal' && row.kind !== 'web') continue;
    // A public ID can return as a different kind or a new web lifetime.
    row.key = JSON.stringify([row.kind || 'note', row.id, row.kind === 'web' ? row.session : null]);
    const old = notes.get(row.key);
    if (row.bodyToken && row.html === undefined) {
      if (old?.bodyToken !== row.bodyToken) throw new Error('Missing cached body');
      row.html = old.html;
    }
    row.bodyText = '';
    if (!row.kind && !row.capture) {
      row.bodyText = old && old.html === row.html ? old.bodyText : bodyText(row.html);
    }
    row.titleText = old && old.title === row.title ? old.titleText : normalize(row.title);
    row.sourceText = old && old.sourceTitle === row.sourceTitle ? old.sourceText : normalize(row.sourceTitle);
    next.set(row.key, row);
  }
  return next;
}
function matches(note) {
  return !query || note.titleText.includes(query) || note.sourceText.includes(query) || note.bodyText.includes(query);
}
function sameImage(a, b) {
  return b?.capture && a.id === b.id && a.imageToken === b.imageToken && a.revision === b.revision;
}
function sameDetail(a, b) {
  if (!a || !b) return a === b;
  return a.key === b.key && a.title === b.title && a.html === b.html
    && Boolean(a.capture) === Boolean(b.capture) && a.imageToken === b.imageToken && a.revision === b.revision;
}
function render(scroll = el('list').parentElement.scrollTop, verifyImage = false) {
  const list = el('list'), panel = list.parentElement;
  const active = document.activeElement;
  const rows = [...notes.values()].filter(matches);
  const ids = new Set(rows.map(row => row.key));
  for (const [id, button] of buttons) {
    if (!ids.has(id)) { button.remove(); buttons.delete(id); }
  }
  for (const [index, row] of rows.entries()) {
    let button = buttons.get(row.key);
    if (!button) {
      button = document.createElement('button');
      const key = row.key;
      button.dataset.id = row.id;
      button.dataset.key = key;
      button.onclick = () => { if (notes.has(key)) { selected = key; render(); } };
      if (row.kind === 'web') {
        button.className = 'web-row';
        const name = document.createElement('span');
        const kind = document.createElement('span');
        window.jajanLanguage.bind(kind, 'textContent', '웹');
        button.append(name); button.append(kind);
      }
      buttons.set(key, button);
    }
    if (row.kind === 'web') {
      if (button.title !== row.title) button.children[0].textContent = Array.from(row.title).slice(0, 30).join('');
    } else if (button.textContent !== (row.title || '')) button.textContent = row.title || '';
    if (button.title !== (row.title || row.id)) button.title = row.title || row.id;
    const current = String(selected === row.key);
    if (button.getAttribute('aria-current') !== current) button.setAttribute('aria-current', current);
    if (list.children[index] !== button) list.insertBefore(button, list.children[index] || null);
  }
  const status = !notes.size ? '아직 메모가 없습니다.' : !rows.length ? '검색 결과가 없습니다.' : '';
  if (el('list-status').textContent !== status) window.jajanLanguage.bind(el('list-status'), 'textContent', status);
  if (active?.dataset.id && active !== document.activeElement) {
    (buttons.get(active.dataset.key) || search).focus({preventScroll:true});
  }
  panel.scrollTop = scroll;
  const next = ids.has(selected) ? notes.get(selected) : null;
  const target = el('detail');
  const placeholder = notes.size ? '메모를 선택하세요.' : '아직 메모가 없습니다.';
  if (captureView && next?.capture && displayed?.key === next.key) {
    if (displayed.title !== next.title) {
      captureView.title.textContent = next.title || next.id;
      if (captureView.image) captureView.image.alt = captureView.title.textContent;
    }
    const reload = verifyImage || !sameImage(displayed, next);
    displayed = next;
    syncVisibility();
    if (reload) loadImage(next, captureView, verifyImage);
    return;
  }
  if (sameDetail(displayed, next) && (next || target.textContent === window.jajanLanguage.t(placeholder))) {
    syncVisibility();
    return;
  }
  const hadFocus = active && target.contains(active);
  const hadToggleFocus = active && active === visibilityView?.button;
  const detailScroll = next && displayed?.key === next.key ? target.scrollTop : 0;
  displayed = next;
  detail(next);
  target.scrollTop = detailScroll;
  if (hadFocus && document.activeElement !== active) {
    const focusTarget = hadToggleFocus && visibilityView ? visibilityView.button : target;
    focusTarget.focus({preventScroll:true});
  }
}
function applyQuery() {
  const next = normalize(search.value);
  if (query === next) return;
  const scroll = el('list').parentElement.scrollTop;
  if (!query) unfilteredScroll = scroll;
  query = next;
  render(query ? scroll : unfilteredScroll);
}
search.addEventListener('compositionstart', () => { composing = true; });
search.addEventListener('compositionend', () => { composing = false; applyQuery(); });
search.addEventListener('input', event => { if (!composing && !event.isComposing) applyQuery(); });
search.addEventListener('keydown', event => {
  if (event.key === 'Escape' && !composing && !event.isComposing && search.value) {
    event.preventDefault();
    search.value = '';
    applyQuery();
  }
});
async function refresh(force = false) {
  if(window.jajanPrivacy && !window.jajanPrivacy.accepted())return;
  if (busy || visibilityPending.size) { if (force) retryFull = true; return; }
  busy = true;
  const version = visibilityVersion;
  force ||= retryFull;
  retryFull = false;
  try {
    const knownBodies = Object.fromEntries([...notes.values()].filter(n => n.bodyToken).map(n => [n.id, n.bodyToken]));
    const rows = await invoke('manager_notes', {knownBodies:force ? {} : knownBodies});
    if (version !== visibilityVersion) return;
    notes = readRows(rows);
    if (!notes.has(selected)) selected = null;
    if (cachedImage && !sameImage(cachedImage.note, notes.get(cachedImage.note.key))) cachedImage = null;
    listFailed = false;
    updateError();
    // Explicit refresh also detects file rewrites that kept the same stamp.
    render(undefined, force);
  } catch { listFailed = true; retryFull = true; updateError(); }
  finally { busy = false; }
}
// Content links remain inert; showing a memo requires an explicit button.
el('detail').addEventListener('click', event => { if (event.target.closest('a')) event.preventDefault(); });
window.addEventListener('focus', () => refresh());
window.addEventListener('jajan-language-change', () => { if (!el('notes-page').hidden) refresh(); });
setInterval(() => { if (!document.hidden && !el('notes-page').hidden) refresh(); }, 2000);
if(window.jajanPrivacy){
  window.jajanPrivacy.onAccepted=()=>page(false);
  window.jajanPrivacy.ready.then(()=>{if(window.jajanPrivacy.accepted())page(false);}).catch(()=>{});
}else refresh();

el('app-language').onchange = async event => {
  const select = event.currentTarget;
  select.disabled = true;
  el('language-error').hidden = true;
  try {
    const saved = await invoke('language_save', {choice: select.value});
    window.jajanLanguage.apply(saved);
    if (saved.refresh_failed) {
      window.jajanLanguage.bind(el('language-error'), 'textContent', '언어는 저장했지만 일부 화면에 적용하지 못했습니다.');
      el('language-error').hidden = false;
    }
  } catch {
    select.value = window.jajanLanguage.choice;
    window.jajanLanguage.bind(el('language-error'), 'textContent', '언어 설정을 저장하지 못했습니다.');
    el('language-error').hidden = false;
  } finally { select.disabled = false; }
};
