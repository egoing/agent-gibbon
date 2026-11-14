import { toggleMark, setBlockType } from '@milkdown/prose/commands';

// This toolbar edits the existing ProseMirror selection; it never owns a draft.
export function createFormatMenu(view, root, enabled) {
  let dismissedSelection;
  const menu = document.createElement('div');
  menu.id = 'selection-format'; menu.className = 'selection-format'; menu.hidden = true;
  menu.setAttribute('role', 'toolbar'); menu.setAttribute('aria-label', '선택한 글자 서식');
  const actions = [
    ['굵게', 'B', 'strong'], ['기울임', 'I', 'emphasis'], ['취소선', 'S', 'strike_through'],
    ['코드', '</>', 'inlineCode'], ['제목 1', 'H1', 1], ['제목 2', 'H2', 2], ['본문', 'P', 0],
  ];
  const buttons = actions.map(([label, text, type]) => {
    const button = document.createElement('button');
    button.type = 'button'; button.textContent = text; button.title = label;
    button.setAttribute('aria-label', label); button.dataset.format = String(type);
    button.addEventListener('mousedown', event => event.preventDefault());
    button.addEventListener('click', () => {
      if (!enabled()) return;
      const command = typeof type === 'number'
        ? setBlockType(type ? view.state.schema.nodes.heading : view.state.schema.nodes.paragraph, type ? { level: type } : undefined)
        : toggleMark(view.state.schema.marks[type]);
      command(view.state, view.dispatch, view);
      view.focus(); update();
    });
    menu.appendChild(button); return button;
  });
  document.body.appendChild(menu);
  const viewport = root.parentElement;
  function update() {
    const { selection, doc, schema } = view.state;
    if (!enabled() || !document.hasFocus() || selection.empty || dismissedSelection?.eq(selection) || !view.hasFocus() && !menu.contains(document.activeElement) || !view.dom.getClientRects().length || !doc.textBetween(selection.from, selection.to).trim()) {
      menu.hidden = true; return;
    }
    const bounds = viewport.getBoundingClientRect();
    const first = view.coordsAtPos(selection.from), last = view.coordsAtPos(selection.to);
    if (bounds.height < 26 || last.bottom < bounds.top || first.top > bounds.bottom) { menu.hidden = true; return; }
    menu.style.maxWidth = `${Math.max(0, bounds.width - 8)}px`;
    menu.hidden = false;
    buttons.forEach((button, index) => {
      const type = actions[index][2];
      const command = typeof type === 'number' ? setBlockType(type ? schema.nodes.heading : schema.nodes.paragraph, type ? { level: type } : undefined) : toggleMark(schema.marks[type]);
      button.disabled = !command(view.state);
      const active = typeof type === 'number'
        ? selection.$from.parent.type === (type ? schema.nodes.heading : schema.nodes.paragraph) && (!type || selection.$from.parent.attrs.level === type)
        : doc.rangeHasMark(selection.from, selection.to, schema.marks[type]);
      button.setAttribute('aria-pressed', String(active));
    });
    const rect = menu.getBoundingClientRect();
    const left = Math.max(bounds.left + 4, Math.min((first.left + last.left - rect.width) / 2, bounds.right - rect.width - 4));
    const preferredTop = first.top - rect.height - 4 >= bounds.top ? first.top - rect.height - 4 : last.bottom + 4;
    menu.style.left = `${left}px`;
    menu.style.top = `${Math.max(bounds.top, Math.min(preferredTop, bounds.bottom - rect.height))}px`;
  }
  function hide() { menu.hidden = true; }
  menu.addEventListener('keydown', event => {
    if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); dismissedSelection = view.state.selection; hide(); view.focus(); return; }
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const available = buttons.filter(button => !button.disabled);
    const current = available.indexOf(document.activeElement);
    const index = event.key === 'Home' ? 0 : event.key === 'End' ? available.length - 1 : (current + (event.key === 'ArrowRight' ? 1 : -1) + available.length) % available.length;
    available[index]?.focus(); available[index]?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  });
  function keyboard(event) {
    if (event.altKey && event.key === 'F10' && !menu.hidden) { event.preventDefault(); buttons.find(button => !button.disabled)?.focus(); }
  }
  view.dom.addEventListener('keydown', keyboard);
  document.addEventListener('selectionchange', update);
  document.addEventListener('focusin', update);
  viewport.addEventListener('scroll', update, { passive: true });
  window.addEventListener('resize', update); window.addEventListener('blur', hide);
  const observer = new MutationObserver(update);
  observer.observe(document.body, { attributes: true, attributeFilter: ['class'] });
  return { update, hide, destroy() {
    menu.remove(); observer.disconnect(); view.dom.removeEventListener('keydown', keyboard);
    document.removeEventListener('selectionchange', update); document.removeEventListener('focusin', update);
    viewport.removeEventListener('scroll', update); window.removeEventListener('resize', update); window.removeEventListener('blur', hide);
  } };
}
