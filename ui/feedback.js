// Inline companion: rendering never changes the native window geometry.
export function createFeedback(doc, action) {
  const language = doc.defaultView.jajanLanguage;
  const surface = doc.getElementById('feedback');
  const message = doc.getElementById('message');
  const actions = doc.getElementById('actions');
  let signature;
  let currentView;
  return (view, suppressed = false) => {
    surface.hidden = suppressed || (!view.message && !view.buttons.length);
    const next = JSON.stringify(view);
    if (signature === next) return;
    currentView = view;
    surface.classList.toggle('error', !!view.error);
    message.setAttribute('role', view.error ? 'alert' : 'status');
    language.bind(message, 'textContent', view.message);
    // Reconcile in place so a notice update preserves keyboard focus.
    for (const old of [...actions.children]) if (!view.buttons.some(b => b.id === old.dataset.action)) old.remove();
    for (const item of view.buttons) {
      let button = [...actions.children].find(b => b.dataset.action === item.id);
      if (!button) {
        button = doc.createElement('button');
        button.dataset.action = item.id;
        button.addEventListener('click', () => {
          const current = currentView.buttons.find(b => b.id === item.id);
          if (current && !current.disabled) action(item.id, current.revision ?? currentView.revision);
        });
        actions.appendChild(button);
      }
      const shortcut = /Mac|iPhone|iPad/.test(doc.defaultView.navigator.platform) ? 'Command S' : 'Ctrl S';
      const label = item.id === 'save' ? language.msg('{label} · {shortcut}', {label: language.msg(item.label), shortcut}) : item.label;
      language.bind(button, 'title', label);
      language.bind(button, 'aria-label', label);
      language.bind(button, 'textContent', item.label);
      button.disabled = item.disabled;
      button.classList.toggle('primary', !!item.primary);
    }
    signature = next;
  };
}
