// Both toolbar states reserve the same row above content.
const controls = document.getElementById('controls');
const items = document.getElementById('toolbar-items');
const previous = document.getElementById('toolbar-previous');
const next = document.getElementById('toolbar-next');
const toggle = document.getElementById('toolbar-toggle');
const title = document.getElementById('chrome-title');
const sourceTitle = document.getElementById('fold-title');
function updateTitle() {
  title.textContent = sourceTitle?.textContent || window.gibbonToolbarTitle || '메모';
  title.title = title.textContent;
}
if (sourceTitle) new MutationObserver(updateTitle).observe(sourceTitle, { childList: true, characterData: true, subtree: true });
if (sourceTitle) updateTitle();
let changingFold = false;
function displayFold(collapsed) {
  document.body.classList.toggle('toolbar-collapsed', collapsed);
  toggle.setAttribute('aria-expanded', String(!collapsed));
  toggle.setAttribute('aria-label', collapsed ? '메모 펼치기' : '메모 접기');
  toggle.title = collapsed ? '메모 펼치기' : '메모 접기';
  refresh();
}
window.gibbonToolbarFold = displayFold;
toggle.addEventListener('click', async () => {
  if (changingFold) return;
  changingFold = true;
  try {
    if (window.gibbonToggleNote) await window.gibbonToggleNote();
    else displayFold(await window.__TAURI__.core.invoke('chrome_fold'));
  } catch (error) {
    const status = document.getElementById('status');
    if (status) status.textContent = String(error);
  } finally {
    changingFold = false;
  }
});
function refresh() {
  const overflow = !document.body.classList.contains('folded') && !document.body.classList.contains('toolbar-collapsed') &&
    items.scrollWidth > controls.clientWidth - toggle.offsetWidth - document.getElementById('close').offsetWidth - 12;
  previous.hidden = next.hidden = !overflow;
  previous.setAttribute('aria-disabled', String(items.scrollLeft <= 1));
  next.setAttribute('aria-disabled', String(items.scrollLeft + items.clientWidth >= items.scrollWidth - 1));
}
function page(direction) {
  const buttons = [...items.querySelectorAll('button')].filter(button => button.getClientRects().length);
  const left = items.getBoundingClientRect().left;
  if (direction > 0) {
    const target = buttons.find(button => button.getBoundingClientRect().right > left + items.clientWidth + 1);
    if (target) items.scrollLeft = Math.floor(items.scrollLeft + target.getBoundingClientRect().left - left);
  } else {
    const target = buttons.filter(button => button.getBoundingClientRect().left < left - 1).pop();
    if (target) items.scrollLeft = Math.ceil(items.scrollLeft + target.getBoundingClientRect().right - left - items.clientWidth);
  }
  refresh();
}
previous.addEventListener('click', () => page(-1));
next.addEventListener('click', () => page(1));
items.addEventListener('focusin', event => {
  const rect = event.target.getBoundingClientRect();
  const bounds = items.getBoundingClientRect();
  if (rect.left < bounds.left) items.scrollLeft = Math.floor(items.scrollLeft + rect.left - bounds.left);
  else if (rect.right > bounds.right) items.scrollLeft = Math.ceil(items.scrollLeft + rect.right - bounds.right);
  refresh();
});
items.addEventListener('scroll', refresh);
const resize = new ResizeObserver(refresh);
resize.observe(controls);
resize.observe(items);
new MutationObserver(refresh).observe(document.body, { attributes: true, attributeFilter: ['class'] });
new MutationObserver(refresh).observe(items, { attributes: true, subtree: true, attributeFilter: ['hidden'] });
refresh();

// Let WebView's platform tooltip escape the memo's content bounds.
function syncTooltips() {
  for (const button of controls.querySelectorAll('button')) {
    const label = button.getAttribute('aria-label');
    if (label && button.title !== label) button.title = label;
  }
}
new MutationObserver(syncTooltips).observe(controls, {
  subtree: true, attributes: true, attributeFilter: ['aria-label'],
});
syncTooltips();

// Track pointer entry regardless of keyboard focus.
// Never activate the window to produce hover feedback; first-click policy
// continues to belong to the native window.
const toolbarButtons = [...controls.querySelectorAll('button')];
for (const button of toolbarButtons) {
  button.addEventListener('pointerenter', () => {
    if (!button.disabled) button.setAttribute('data-pointer-over', '');
  });
  button.addEventListener('pointerleave', () => button.removeAttribute('data-pointer-over'));
}
// macOS also samples the native pointer when the WebView is inactive. Hit-test
// the rendered DOM so hidden/paged buttons and resize/drag regions stay intact.
window.__TAURI__.event.listen('toolbar-pointer', ({ payload }) => {
  const target = payload ? document.elementFromPoint(payload.x, payload.y)?.closest('button') : null;
  for (const button of toolbarButtons) {
    if (button === target && !button.disabled) button.setAttribute('data-pointer-over', '');
    else button.removeAttribute('data-pointer-over');
  }
});
