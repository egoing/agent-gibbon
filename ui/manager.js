'use strict';
const invoke = window.__TAURI__.core.invoke;
const el = id => document.getElementById(id);
let notes = [], selected = null, busy = false, detailVersion = 0, signature = '';
function page(settings) {
  el('notes-page').hidden = settings;
  el('settings-page').hidden = !settings;
  el('notes-nav').setAttribute('aria-pressed', String(!settings));
  el('settings-nav').setAttribute('aria-pressed', String(settings));
  if (!settings) refresh();
}
el('notes-nav').onclick = () => page(false);
el('settings-nav').onclick = () => page(true);
el('retry').onclick = () => refresh(true);
el('refresh').onclick = () => refresh(true);
async function detail() {
  const version = ++detailVersion, note = notes.find(n => n.id === selected);
  const target = el('detail'); target.replaceChildren();
  if (!note) { target.textContent = notes.length ? '메모를 선택하세요.' : '아직 메모가 없습니다.'; return; }
  const title = document.createElement('h1'); title.textContent = note.title || note.id; target.append(title);
  if (note.kind) {
    const kind=document.createElement('p');kind.textContent=note.kind==='terminal'?'터미널':'에이전트';target.append(kind);
    const open=document.createElement('button');open.textContent='화면에 띄우기';open.onclick=()=>invoke('manager_terminal',{id:note.id,action:'open'}).catch(()=>{el('error').hidden=false;});target.append(open);
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
    const button = document.createElement('button'); button.textContent = (note.kind ? `[${note.kind==='terminal'?'터미널':'에이전트'}] ` : '') + (note.title || note.id);
    button.title = note.title || note.id;
    button.dataset.id = note.id; button.setAttribute('aria-current', String(note.id === selected));
    button.onclick = () => { selected = note.id; render(); detail(); };
    list.append(button);
    if (focusId === note.id) button.focus({ preventScroll: true });
  }
  list.parentElement.scrollTop = scroll;
}
async function refresh(force = false) {
  if (busy) return;
  busy = true;
  try {
    const next = await invoke('manager_notes'), nextSignature = JSON.stringify(next);
    el('error').hidden = true;
    if (force || signature !== nextSignature) {
      notes = next; signature = nextSignature;
      if (!notes.some(n => n.id === selected)) selected = null;
      render(); await detail();
    }
  } catch { el('error').hidden = false; }
  finally { busy = false; }
}
// Content links remain inert; terminal actions require explicit buttons.
el('detail').addEventListener('click', event => { if (event.target.closest('a')) event.preventDefault(); });
window.addEventListener('focus', () => refresh());
setInterval(() => { if (!document.hidden && !el('notes-page').hidden) refresh(); }, 2000);
refresh();
