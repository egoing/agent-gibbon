// Keep native scrolling; this control only mirrors and manipulates scrollTop.
export function scrollGeometry(view, total, offset) {
  const range = Math.max(0, total - view);
  const track = Math.max(0, view - 4);
  const size = range ? Math.min(track, Math.max(24, track * view / total)) : track;
  return { range, size, travel: track - size, top: range ? Math.max(0, Math.min(range, offset)) / range * (track - size) : 0 };
}

export function installOverlayScrollbar(doc = document) {
  const viewport = doc.getElementById('viewport');
  const bar = doc.getElementById('memo-scrollbar');
  if (!viewport || !bar) return;
  let geometry, timer, frame, drag;
  const enabled = () => !['editing', 'image-note', 'folded', 'toolbar-collapsed'].some(name => doc.body.classList.contains(name));
  function reveal() {
    bar.classList.add('scroll-visible');
    clearTimeout(timer);
    timer = setTimeout(() => bar.classList.remove('scroll-visible'), 1000);
  }
  function update() {
    frame = null;
    geometry = scrollGeometry(viewport.clientHeight, viewport.scrollHeight, viewport.scrollTop);
    const wasHidden = bar.hidden;
    bar.hidden = !enabled() || geometry.range <= 1 || viewport.clientHeight < 8;
    bar.style.top = `${viewport.offsetTop + 2}px`;
    bar.style.height = `${Math.max(0, viewport.clientHeight - 4)}px`;
    bar.style.setProperty('--thumb-size', `${geometry.size}px`);
    bar.style.setProperty('--thumb-top', `${geometry.top}px`);
    bar.setAttribute('aria-valuemax', String(Math.round(geometry.range)));
    bar.setAttribute('aria-valuenow', String(Math.round(viewport.scrollTop)));
    if (wasHidden && !bar.hidden) reveal();
  }
  function schedule() { if (frame == null) frame = requestAnimationFrame(update); }
  viewport.addEventListener('scroll', () => { schedule(); reveal(); }, { passive: true });
  // mousedown must not bubble into the application's native note-drag handler.
  bar.addEventListener('mousedown', event => event.stopPropagation());
  bar.addEventListener('pointerdown', event => {
    if (event.button !== 0) return;
    event.preventDefault(); event.stopPropagation();
    update();
    const local = event.clientY - bar.getBoundingClientRect().top;
    if (local < geometry.top || local > geometry.top + geometry.size) {
      viewport.scrollTop = Math.max(0, Math.min(geometry.range, (local - geometry.size / 2) / (geometry.travel || 1) * geometry.range));
    }
    drag = { y: event.clientY, offset: viewport.scrollTop, range: geometry.range, travel: geometry.travel };
    bar.setPointerCapture(event.pointerId);
    bar.classList.add('scroll-dragging');
    reveal();
  });
  bar.addEventListener('pointermove', event => {
    if (!drag) return;
    viewport.scrollTop = drag.offset + (event.clientY - drag.y) * drag.range / (drag.travel || 1);
  });
  function finish() { drag = null; bar.classList.remove('scroll-dragging'); reveal(); }
  bar.addEventListener('lostpointercapture', finish);
  bar.addEventListener('pointerup', finish);
  bar.addEventListener('pointercancel', finish);
  bar.addEventListener('keydown', event => {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    const page = viewport.clientHeight * .9;
    const destinations = { ArrowDown: viewport.scrollTop + 40, ArrowUp: viewport.scrollTop - 40, PageDown: viewport.scrollTop + page, PageUp: viewport.scrollTop - page, Home: 0, End: viewport.scrollHeight };
    if (!(event.key in destinations)) return;
    event.preventDefault(); event.stopPropagation();
    viewport.scrollTop = destinations[event.key]; reveal();
  });
  bar.addEventListener('wheel', event => {
    // Wheel input over the overlay still belongs to the native scroll container.
    if (event.ctrlKey) return;
    const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? viewport.clientHeight : 1;
    viewport.scrollTop += event.deltaY * unit;
    event.preventDefault();
  }, { passive: false });
  new ResizeObserver(schedule).observe(viewport);
  new MutationObserver(schedule).observe(doc.getElementById('content'), { childList: true, subtree: true, characterData: true });
  new MutationObserver(schedule).observe(doc.body, { attributes: true, attributeFilter: ['class'] });
  new MutationObserver(schedule).observe(doc.documentElement, { attributes: true, attributeFilter: ['style'] });
  doc.addEventListener('load', schedule, true);
  doc.fonts?.ready.then(schedule);
  update();
}

if (typeof document !== 'undefined') installOverlayScrollbar();
