// Inline companion: rendering never changes the native window geometry.
export function createFeedback(doc, action) {
  const surface = doc.getElementById('feedback');
  const message = doc.getElementById('message');
  const actions = doc.getElementById('actions');
  let signature;
  let revision = null;
  return (view, suppressed = false) => {
    surface.hidden = suppressed || (!view.message && !view.buttons.length);
    const next = JSON.stringify(view);
    if (signature === next) return;
    revision = view.revision;
    surface.classList.toggle('error', !!view.error);
    message.setAttribute('role', view.error ? 'alert' : 'status');
    message.textContent = view.message;
    // Reconcile in place so a notice update preserves keyboard focus.
    for (const old of [...actions.children]) if (!view.buttons.some(b => b.id === old.dataset.action)) old.remove();
    for (const item of view.buttons) {
      let button = [...actions.children].find(b => b.dataset.action === item.id);
      if (!button) {
        button = doc.createElement('button');
        button.dataset.action = item.id;
        button.addEventListener('click', () => action(item.id, revision));
        actions.appendChild(button);
      }
      const shortcut = /Mac|iPhone|iPad/.test(doc.defaultView.navigator.platform) ? 'Command S' : 'Ctrl S';
      button.title = item.id === 'save' ? `${item.label} · ${shortcut}` : item.label;
      button.setAttribute('aria-label', button.title);
      button.textContent = item.label;
      button.disabled = item.disabled;
      button.classList.toggle('primary', !!item.primary);
    }
    signature = next;
  };
}
