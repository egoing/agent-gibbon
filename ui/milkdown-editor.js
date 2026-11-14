import './editor-compat.js';
import { Editor, rootCtx, defaultValueCtx, editorViewCtx, editorViewOptionsCtx, parserCtx, serializerCtx } from '@milkdown/core';
import { commonmark, imageSchema, linkSchema } from '@milkdown/preset-commonmark';
import { gfm } from '@milkdown/preset-gfm';
import { history } from '@milkdown/plugin-history';
import { $prose } from '@milkdown/utils';
import { Plugin, EditorState, TextSelection } from '@milkdown/prose/state';
import { Decoration, DecorationSet } from '@milkdown/prose/view';
import { preservesMarkdown } from './markdown-policy.mjs';
import { createFormatMenu } from './editor-format-menu.js';

// Source attributes remain serializable, but never become requests or navigation.
const inertImage = imageSchema.extendSchema(previous => ctx => ({
  ...previous(ctx), parseDOM: [],
  toDOM: node => ['span', { 'data-inert-image': '', contenteditable: 'false', role: 'img', 'aria-label': '이미지 표시 안 함' }, node.attrs.alt || '이미지 표시 안 함'],
}));
const inertLink = linkSchema.extendSchema(previous => ctx => ({
  ...previous(ctx), parseDOM: [], toDOM: () => ['span', { 'data-inert-link': '' }, 0],
}));

export async function createMemoEditor(root, onChange) {
  let editor, view, baseDoc, baseMarkdown = '', baseRevision = -1;
  let loading = true, locked = false, protectedSource = false, decorations = DecorationSet.empty;
  let composition = false, restoreInputFocus = false, pendingWrite = false;
  let formatMenu;
  const status = () => ({
    dirty: !!baseDoc && !view.state.doc.eq(baseDoc),
    composing: composition || !!view?.composing,
    protected: protectedSource, revision: baseRevision,
  });
  const changed = () => { if (!loading && view) onChange(status()); };
  function taskView(node, editorView, getPos) {
    const dom = document.createElement('li');
    const button = document.createElement('button');
    button.type = 'button'; button.className = 'editor-task';
    button.contentEditable = 'false'; button.setAttribute('role', 'checkbox');
    button.setAttribute('aria-label', '완료 상태');
    const contentDOM = document.createElement('div');
    dom.append(button, contentDOM);
    function update(next) {
      if (next.type !== node.type) return false;
      node = next;
      const task = typeof node.attrs.checked === 'boolean';
      button.hidden = !task; dom.classList.toggle('task-item', task);
      dom.dataset.itemType = task ? 'task' : 'list';
      dom.dataset.checked = String(node.attrs.checked);
      button.setAttribute('aria-checked', String(!!node.attrs.checked));
      button.disabled = locked || protectedSource;
      return true;
    }
    update(node);
    button.addEventListener('mousedown', event => event.preventDefault());
    button.addEventListener('click', event => {
      event.preventDefault();
      if (locked || protectedSource || composition || editorView.composing) return;
      const pos = getPos();
      if (typeof pos === 'number') editorView.dispatch(editorView.state.tr.setNodeMarkup(pos, undefined, { ...node.attrs, checked: !node.attrs.checked }));
    });
    return { dom, contentDOM, update, stopEvent: event => button.contains(event.target), ignoreMutation: mutation => mutation.type === 'attributes' && (mutation.target === dom || mutation.target === button) };
  }
  const integration = $prose(() => new Plugin({
    filterTransaction: transaction => !transaction.docChanged || (!locked && !protectedSource),
    props: {
      decorations: () => decorations,
      handlePaste(editorView, event) {
        event.preventDefault();
        if (!locked && !protectedSource && !composition && !editorView.composing) {
          const text = event.clipboardData?.getData('text/plain') || '';
          if (text) editorView.dispatch(editorView.state.tr.insertText(text));
        }
        return true;
      },
      handleDrop(_view, event) { event.preventDefault(); return true; },
      handleDOMEvents: {
        compositionstart() { composition = true; formatMenu?.hide(); changed(); return false; },
        compositionend() { composition = false; setTimeout(changed, 50); return false; },
      },
    },
    view: () => ({ update(current, previous) {
      if (!current.state.doc.eq(previous.doc)) {
        decorations = DecorationSet.empty;
        changed();
      }
      formatMenu?.update();
    } }),
  }));
  editor = await Editor.make().config(ctx => {
    ctx.set(rootCtx, root); ctx.set(defaultValueCtx, '');
    ctx.update(editorViewOptionsCtx, options => ({
      ...options, editable: () => !locked && !protectedSource,
      attributes: { role: 'textbox', 'aria-label': '메모 본문', 'aria-multiline': 'true', spellcheck: 'false' },
      nodeViews: { list_item: taskView },
    }));
  }).use(commonmark).use(gfm).use(inertImage).use(inertLink).use(history).use(integration).create();
  view = editor.action(ctx => ctx.get(editorViewCtx));
  view.dom.id = 'memo-editor';
  formatMenu = createFormatMenu(view, root, () => !locked && !protectedSource && !status().composing);
  loading = false;

  function load(note) {
    const scroll = root.parentElement.scrollTop;
    loading = true;
    try {
      let doc = editor.action(ctx => ctx.get(parserCtx)(note.markdown));
      if (!doc) throw new Error('메모 원문을 읽지 못했습니다');
      const serialized = editor.action(ctx => ctx.get(serializerCtx)(doc));
      protectedSource = !preservesMarkdown(note.markdown, serialized);
      if (protectedSource) doc = view.state.schema.node('doc', null, [view.state.schema.node('code_block', { language: '' }, note.markdown ? view.state.schema.text(note.markdown) : undefined)]);
      const position = value => doc.resolve(Math.min(value, doc.content.size));
      const selection = TextSelection.between(position(view.state.selection.anchor), position(view.state.selection.head));
      // A new external base has a new undo history. Never undo across revisions.
      view.updateState(EditorState.create({ doc, schema: view.state.schema, plugins: view.state.plugins, selection }));
      baseDoc = doc; baseMarkdown = note.markdown; baseRevision = note.revision;
      decorations = DecorationSet.empty;
      view.setProps({ editable: () => !locked && !protectedSource });
      view.dom.setAttribute('aria-readonly', String(protectedSource || locked));
      formatMenu.hide();
      root.parentElement.scrollTop = scroll;
    } finally { loading = false; }
    return status();
  }
  return {
    status,
    sync(note) {
      if (baseRevision === note.revision) return status();
      if (status().dirty || status().composing || locked || pendingWrite) return { ...status(), conflict: true };
      if (baseRevision >= 0 && baseMarkdown === note.markdown) { baseRevision = note.revision; return status(); }
      return load(note);
    },
    discard: note => load(note),
    markdown() {
      if (status().composing) throw new Error('한글 조합을 마친 뒤 저장해 주세요');
      if (protectedSource) throw new Error('보존할 수 없는 문법이 있어 원문을 보호하고 있습니다');
      return status().dirty ? editor.action(ctx => ctx.get(serializerCtx)(view.state.doc)) : baseMarkdown;
    },
    capture() { return { doc: view.state.doc, markdown: this.markdown(), revision: baseRevision }; },
    pending(value) { pendingWrite = value; },
    saved(note, snapshot) {
      baseDoc = snapshot?.doc || view.state.doc; baseMarkdown = note.markdown; baseRevision = note.revision;
      decorations = DecorationSet.empty;
      view.setProps({});
      changed();
    },
    lock(value) {
      if (value) restoreInputFocus = view.hasFocus();
      locked = value;
      view.setProps({ editable: () => !locked && !protectedSource });
      view.dom.setAttribute('aria-readonly', String(locked || protectedSource));
      root.querySelectorAll('.editor-task').forEach(button => { button.disabled = locked || protectedSource; });
      if (!value && restoreInputFocus && document.activeElement === document.body && document.hasFocus()) view.focus();
      if (!value) restoreInputFocus = false;
      formatMenu.update();
    },
    highlight(html) {
      if (status().dirty || status().composing || protectedSource) return;
      // Rust owns the sanitized diff. Map visible text to document positions;
      // deleted text is an inert decoration, never part of the editable document.
      const source = document.createElement('div'); source.innerHTML = html;
      const positions = [], chars = [];
      view.state.doc.descendants((node, pos) => {
        if (node.isText) for (let i = 0; i < node.text.length; i++) { chars.push(node.text[i]); positions.push(pos + i); }
      });
      const text = chars.join('');
      let cursor = 0;
      const marks = [];
      const taskPositions = [];
      view.state.doc.descendants((node, pos) => { if (node.type.name === 'list_item' && typeof node.attrs.checked === 'boolean') taskPositions.push({ node, pos }); });
      let taskIndex = 0;
      function visit(node, kind = '') {
        if (node.nodeType === 1 && node.classList.contains('change-removed')) {
          const anchor = positions[Math.min(cursor, positions.length - 1)] || 1;
          const value = node.textContent;
          marks.push(Decoration.widget(anchor, () => {
            const span = document.createElement('span'); span.className = 'change-removed'; span.contentEditable = 'false'; span.textContent = value; return span;
          }, { side: -1, ignoreSelection: true }));
          return;
        }
        if (node.nodeType === 1) {
          if (node.tagName === 'MARK') kind = node.className;
          if (node.classList.contains('task-checkbox')) {
            const task = taskPositions[taskIndex++];
            if (task && kind) marks.push(Decoration.node(task.pos, task.pos + task.node.nodeSize, { class: kind }));
            return;
          }
          for (const child of node.childNodes) visit(child, kind);
        } else if (node.nodeType === 3) {
          const value = node.textContent;
          if (!value.trim()) return;
          const start = text.indexOf(value, cursor);
          if (start >= 0) {
            if (kind) marks.push(Decoration.inline(positions[start], positions[start + value.length - 1] + 1, { class: kind }));
            cursor = start + value.length;
          }
        }
      }
      visit(source);
      decorations = DecorationSet.create(view.state.doc, marks);
      view.setProps({});
    },
    focus: () => view.focus(),
    destroy: () => { formatMenu.destroy(); return editor.destroy(); },
  };
}
