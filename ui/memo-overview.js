const controls = document.querySelector('nav');
const status = document.querySelector('[role=status]');
let generation = window.overviewGeneration || 0;
let pending = false;

const buttons = [...controls.querySelectorAll('button')];
function highlight(target) {
  for (const button of buttons) button.toggleAttribute('data-pointer-over', button === target);
}
window.__TAURI__.webviewWindow.getCurrentWebviewWindow().listen('toolbar-pointer', ({ payload }) => {
  highlight(payload && document.elementFromPoint(payload.x, payload.y)?.closest('button'));
});

window.beginOverview = (id) => {
  highlight(null);
  generation = id;
  pending = false;
  status.hidden = true;
};

controls.addEventListener('click', async (event) => {
  const button = event.target.closest('button[data-action]');
  if (!button || pending || !generation) return;
  const visit = generation;
  pending = true;
  status.hidden = true;
  try {
    await window.jajanInvoke('overview_action', { action: button.dataset.action, generation: visit });
  } catch (error) {
    if (visit === generation) {
      window.jajanLanguage.bind(status, 'textContent', String(error));
      status.hidden = false;
    }
  } finally {
    if (visit === generation) pending = false;
  }
});
