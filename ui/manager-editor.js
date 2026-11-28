// One edit session owns one memo ID from creation through its last pending save.
function managerEditor(id, target, tools, invoke) {
  tools.classList.add('manager-edit-actions');
  const root = document.createElement('div');
  root.className = 'manager-editor';
  root.hidden = true;
  const section = document.createElement('div');
  section.append(root);
  target.append(section);
  const saveButton = document.createElement('button');
  const cancelButton = document.createElement('button');
  const message = document.createElement('span');
  message.setAttribute('role', 'status');
  const label = window.jajanLanguage.bind;
  label(saveButton, 'textContent', '저장');
  label(cancelButton, 'textContent', '변경 취소');
  tools.append(saveButton, cancelButton, message);

  let editor, pending, timer, loading = false, disposed = false, problem = '';
  let initialized = false;
  function state() { return editor?.status() || {}; }
  function paint() {
    const s = state();
    saveButton.disabled = !s.dirty || s.composing || !!pending || loading;
    cancelButton.hidden = !s.dirty || !problem;
    cancelButton.disabled = !!pending || loading || s.composing;
    const status = problem || (pending ? '저장 중' : s.protected ? '보존할 수 없는 문법이 있어 원문을 보호하고 있습니다' : '');
    label(message, 'textContent', status);
  }
  function changed() {
    clearTimeout(timer);
    paint();
    if (initialized && state().dirty && !state().composing && !pending && !problem)
      timer = setTimeout(flush, 650);
  }
  async function flush() {
    clearTimeout(timer);
    while (pending) {
      if (!await pending) return false;
    }
    if (state().composing) return false;
    if (!state().dirty) return true;
    const snapshot = editor.capture();
    editor.pending(true);
    pending = invoke('save_note', {id, markdown:snapshot.markdown, expectedRevision:snapshot.revision})
      .then(note => { problem = ''; editor.saved(note, snapshot); return true; })
      .catch(error => { problem = String(error); return false; })
      .finally(() => { pending = null; editor.pending(false); changed(); });
    paint();
    return await pending && (!state().dirty || flush());
  }
  async function read(revision, discard = false) {
    if (disposed || loading || pending || (initialized && !discard && state().revision >= Number(revision))) return;
    loading = true;
    if (discard) editor.lock(true);
    try {
      const note = await invoke('manager_text', {id});
      if (disposed) return;
      if (!editor) {
        const {createMemoEditor} = await import('./generated/milkdown.js');
        if (disposed) return;
        editor = await createMemoEditor(root, changed);
        if (disposed) { editor.destroy(); return; }
      }
      if (pending || state().revision > note.revision) return;
      if (discard) editor.discard(note);
      else if (editor.sync(note).conflict) {
        problem = '메모가 외부에서 갱신되었습니다. 초안을 복사한 뒤 취소하면 최신 내용을 확인할 수 있습니다.';
        clearTimeout(timer);
        return;
      }
      initialized = true;
      root.hidden = false;
      problem = '';
    } catch (error) { if (!disposed) problem = String(error); }
    finally {
      loading = false;
      if (!disposed) { if (discard) editor.lock(false); paint(); }
    }
  }
  saveButton.onclick = flush;
  cancelButton.onclick = () => read(null, true);
  root.onkeydown = event => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
      event.preventDefault(); flush();
    }
  };
  paint();
  return {
    read, flush,
    busy: () => !!pending || !!state().dirty || !!state().composing,
    activity: () => ({editing: !!state().dirty || !!state().composing, saving: !!pending}),
    dispose() { disposed = true; clearTimeout(timer); editor?.destroy(); },
  };
}
