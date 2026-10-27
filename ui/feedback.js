import { themeFor } from './theme.mjs';
const { invoke } = window.__TAURI__.core;
const surface = document.getElementById('feedback');
const message = document.getElementById('message');
const actions = document.getElementById('actions');
let signature;
let renderedRevision = null;
async function refresh() {
  try {
    const state = await invoke('feedback_snapshot');
    const next = JSON.stringify(state.view);
    if (signature !== next) {
      const view = state.view;
      renderedRevision = view.revision;
      const theme = themeFor(view.color);
      for (const [key, value] of Object.entries({ background: view.color, foreground: theme.foreground, border: theme.border, primary: theme.panel, error: theme.error })) document.documentElement.style.setProperty(`--${key}`, value);
      document.documentElement.style.colorScheme = theme.scheme;
      surface.classList.toggle('error', view.error);
      message.setAttribute('role', view.error ? 'alert' : 'status');
      message.textContent = view.message;
      // Reconcile buttons so polling and notices never discard keyboard focus.
      for (const old of [...actions.children]) if (!view.buttons.some(b => b.id === old.dataset.action)) old.remove();
      for (const item of view.buttons) {
        let button = [...actions.children].find(b => b.dataset.action === item.id);
        if (!button) {
          button = document.createElement('button'); button.dataset.action = item.id;
          button.addEventListener('click', () => invoke('feedback_action', { action: item.id, revision: renderedRevision }).catch(report));
          actions.appendChild(button);
        }
        button.textContent = item.label; button.disabled = item.disabled; button.classList.toggle('primary', !!item.primary);
      }
      signature = next;
    }
    // Measure against the memo width, never against the previous narrow Toast.
    surface.style.width = 'max-content';
    surface.style.maxWidth = `${state.max_width}px`;
    document.body.style.width = `${state.max_width}px`;
    const rect = surface.getBoundingClientRect();
    // Fill the rounded native width: max-content can leave a fractional blank edge.
    surface.style.width = `${Math.ceil(rect.width)}px`;
    document.body.style.width = `${Math.ceil(rect.width)}px`;
    const layout = await invoke('layout_feedback', { width: Math.ceil(rect.width), height: Math.ceil(rect.height) });
    surface.dataset.edge = layout.edge;
  } catch (error) { report(error); }
  setTimeout(refresh, 100);
}
function report(error) { console.error('Feedback:', error); }
document.addEventListener('keydown', event => {
  if (event.key === 'Escape') invoke('feedback_action', { action: 'cancel', revision: renderedRevision }).catch(report);
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') { event.preventDefault(); invoke('feedback_action', { action: 'export', revision: renderedRevision }).catch(report); }
});
refresh();
