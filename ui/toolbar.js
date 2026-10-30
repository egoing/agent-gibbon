// Paging changes only the overlay; never resize or refit the Memo Content.
const controls = document.getElementById('controls');
const items = document.getElementById('toolbar-items');
const previous = document.getElementById('toolbar-previous');
const next = document.getElementById('toolbar-next');
function refresh() {
  const overflow = !document.body.classList.contains('folded') &&
    items.scrollWidth > controls.clientWidth - 2;
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
