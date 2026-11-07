'use strict';
const invoke = window.gibbonInvoke || window.__TAURI__.core.invoke;
const el = id => document.getElementById(id);
let notes = [], selected = null, busy = false, detailVersion = 0, signature = '';
let visibilityView = null, visibilityVersion = 0;
const visibilityPending = new Set();
function syncVisibility() {
  if (!visibilityView) return;
  const note = notes.find(n => n.id === visibilityView.id);
  if (!note) return;
  visibilityView.button.textContent = note.visible ? '화면에서 숨기기' : '화면에 띄우기';
  visibilityView.button.disabled = visibilityPending.has(note.id);
}
// Register settings destinations here; navigation preserves each panel's DOM.
const settingsSections = ['privacy', 'skills', 'appearance', ...(el('settings-updater') ? ['updater'] : [])];
let selectedSettingsSection = 'privacy';
function settingsSection(selectedSection) {
  selectedSettingsSection = selectedSection;
  for (const section of settingsSections) {
    const active = section === selectedSection;
    el(`settings-${section}`).hidden = !active;
    el(`settings-${section}-nav`).setAttribute('aria-current', active ? 'page' : 'false');
  }
  if (selectedSection === 'updater') window.gibbonUpdater?.refresh();
}
for (const section of settingsSections) {
  el(`settings-${section}-nav`).onclick = () => {
    settingsSection(section);
    if (section === 'skills') window.gibbonSkills?.refresh();
  };
}
function page(settings) {
  if(window.gibbonPrivacy && !window.gibbonPrivacy.accepted()){window.gibbonPrivacy.showWelcome();return;}
  el('notes-page').hidden = settings;
  el('settings-page').hidden = !settings;
  el('notes-nav').setAttribute('aria-pressed', String(!settings));
  el('settings-nav').setAttribute('aria-pressed', String(settings));
  if (!settings) refresh();
  else if (selectedSettingsSection === 'skills') window.gibbonSkills?.refresh();
  else if (selectedSettingsSection === 'updater') window.gibbonUpdater?.refresh();
  else window.gibbonPrivacy?.refresh().catch(()=>{});
}
window.gibbonShowNotes = () => page(false);
el('notes-nav').onclick = window.gibbonShowNotes;
el('settings-nav').onclick = () => page(true);
el('retry').onclick = () => refresh(true);
el('refresh').onclick = () => refresh(true);
async function detail() {
  const version = ++detailVersion, note = notes.find(n => n.id === selected);
  const target = el('detail'); target.replaceChildren();
  visibilityView = null;
  if (!note) { target.textContent = notes.length ? '메모를 선택하세요.' : '아직 메모가 없습니다.'; return; }
  const tools = document.createElement('div'); tools.className = 'detail-tools';
  tools.setAttribute('role', 'group'); tools.setAttribute('aria-label', '메모 공통 도구');
  const open = document.createElement('button');
  const error = document.createElement('p'); error.className = 'detail-error'; error.hidden = true; error.setAttribute('role', 'alert');
  open.onclick = async () => {
    if (visibilityPending.has(note.id)) return;
    const desired = !notes.find(n => n.id === note.id)?.visible;
    visibilityPending.add(note.id); visibilityVersion++; syncVisibility(); error.hidden = true;
    try {
      const visible = await invoke('manager_set_visibility', { id: note.id, visible: desired });
      const current = notes.find(n => n.id === note.id);
      if (current) current.visible = visible;
    }
    catch { if (version === detailVersion) { error.textContent = '메모 표시 상태를 변경하지 못했습니다. 다시 시도하세요.'; error.hidden = false; } }
    finally { visibilityPending.delete(note.id); visibilityVersion++; syncVisibility(); }
  };
  visibilityView = { id: note.id, button: open }; syncVisibility();
  tools.append(open); tools.append(error); target.append(tools);
  if (note.kind || note.capture) {
    const title = document.createElement('h1'); title.textContent = note.title || note.id; target.append(title);
  }
  if (note.kind) {
    const kind=document.createElement('p');kind.textContent='터미널';target.append(kind);
    if(note.kind==='terminal'){const remove=document.createElement('button');remove.textContent='목록에서 삭제';remove.onclick=async()=>{try{await invoke('manager_terminal',{id:note.id,action:'delete'});await refresh(true);}catch(e){el('error').hidden=false;remove.title=String(e);}};target.append(remove);}
  } else if (note.capture) {
    try {
      const src = await invoke('manager_image', { id: note.id });
      if (version !== detailVersion) return;
      const image = document.createElement('img'); image.alt = note.title || '메모 이미지'; image.onerror = () => { if (version === detailVersion) el('error').hidden = false; }; image.src = src; target.append(image);
    } catch { if (version === detailVersion) el('error').hidden = false; }
  } else {
    // HTML is rendered and sanitized by the same Rust pipeline as floating memos.
    const content = document.createElement('div'); content.innerHTML = note.html; target.append(content);
  }
}
function render() {
  const list = el('list'), focusId = document.activeElement?.dataset.id;
  const scroll = list.parentElement.scrollTop;
  list.replaceChildren();
  if (!notes.length) list.textContent = '아직 메모가 없습니다.';
  for (const note of notes) {
    const button = document.createElement('button'); button.textContent = note.title;
    button.title = note.title || note.id;
    button.dataset.id = note.id; button.setAttribute('aria-current', String(note.id === selected));
    button.onclick = () => { selected = note.id; render(); detail(); };
    list.append(button);
    if (focusId === note.id) button.focus({ preventScroll: true });
  }
  list.parentElement.scrollTop = scroll;
}
async function refresh(force = false) {
  if(window.gibbonPrivacy && !window.gibbonPrivacy.accepted())return;
  if (busy || visibilityPending.size) return;
  busy = true;
  const version = visibilityVersion;
  try {
    const next = (await invoke('manager_notes')).filter(note => !note.kind || note.kind === 'terminal');
    if (version !== visibilityVersion) return;
    const nextSignature = JSON.stringify(next.map(({ visible, ...note }) => note));
    el('error').hidden = true;
    notes = next;
    if (force || signature !== nextSignature) {
      signature = nextSignature;
      if (!notes.some(n => n.id === selected)) selected = null;
      render(); await detail();
    } else syncVisibility();
  } catch { el('error').hidden = false; }
  finally { busy = false; }
}
// Content links remain inert; showing a memo requires an explicit button.
el('detail').addEventListener('click', event => { if (event.target.closest('a')) event.preventDefault(); });
window.addEventListener('focus', () => refresh());
setInterval(() => { if (!document.hidden && !el('notes-page').hidden) refresh(); }, 2000);
if(window.gibbonPrivacy){
  window.gibbonPrivacy.onAccepted=()=>page(false);
  window.gibbonPrivacy.ready.then(()=>{if(window.gibbonPrivacy.accepted())page(false);}).catch(()=>{});
}else refresh();
