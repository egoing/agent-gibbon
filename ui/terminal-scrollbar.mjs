import {scrollGeometry} from './overlay-scrollbar.js';

// The terminal's public buffer API owns scrollback. Never move a DOM scrollTop:
// alternate-screen apps retain xterm's mouse, wheel and keyboard handling.
export function installTerminalScrollbar(terminal, doc = document) {
  const bar = doc.getElementById('memo-scrollbar');
  const host = doc.getElementById('terminal');
  let geometry, drag, timer, frame;
  const reveal = () => {
    bar.classList.add('scroll-visible');
    clearTimeout(timer);
    timer = setTimeout(() => bar.classList.remove('scroll-visible'), 1000);
  };
  function update() {
    frame = null;
    const buffer = terminal.buffer.active;
    const height = host.clientHeight - parseFloat(getComputedStyle(host).paddingBottom);
    geometry = scrollGeometry(height, height * (buffer.baseY + terminal.rows) / terminal.rows, height * buffer.viewportY / terminal.rows);
    const hidden = bar.hidden;
    bar.hidden = buffer.type !== 'normal' || !buffer.baseY || !height || host.hidden || doc.body.classList.contains('toolbar-collapsed');
    if (bar.hidden) finish();
    bar.style.top = `${host.offsetTop + 2}px`;
    bar.style.height = `${Math.max(0, height - 4)}px`;
    bar.style.setProperty('--thumb-size', `${geometry.size}px`);
    bar.style.setProperty('--thumb-top', `${geometry.top}px`);
    bar.setAttribute('aria-valuemax', String(buffer.baseY));
    bar.setAttribute('aria-valuenow', String(buffer.viewportY));
    if (hidden && !bar.hidden) reveal();
  }
  const schedule = () => { if (frame == null) frame = requestAnimationFrame(update); };
  const scrollTo = line => terminal.scrollToLine(Math.max(0, Math.min(terminal.buffer.active.baseY, Math.round(line))));
  function finish() { drag = null; bar.classList.remove('scroll-dragging'); }
  terminal.onScroll(() => { schedule(); reveal(); });
  terminal.onWriteParsed(schedule);
  terminal.onResize(schedule);
  terminal.buffer.onBufferChange(() => { finish(); schedule(); });
  new ResizeObserver(schedule).observe(host);
  new MutationObserver(schedule).observe(doc.body, {attributes:true, attributeFilter:['class']});
  new MutationObserver(schedule).observe(host, {attributes:true, attributeFilter:['hidden']});
  bar.addEventListener('mousedown', event => event.stopPropagation());
  bar.addEventListener('pointerdown', event => {
    if (event.button !== 0) return;
    event.preventDefault(); event.stopPropagation(); update();
    if (bar.hidden) return;
    const local = event.clientY - bar.getBoundingClientRect().top;
    if (local < geometry.top || local > geometry.top + geometry.size) scrollTo((local - geometry.size / 2) / (geometry.travel || 1) * terminal.buffer.active.baseY);
    drag = {y:event.clientY, line:terminal.buffer.active.viewportY, base:terminal.buffer.active.baseY, travel:geometry.travel};
    bar.setPointerCapture(event.pointerId);
    bar.classList.add('scroll-dragging'); reveal();
  });
  bar.addEventListener('pointermove', event => {
    if (drag) scrollTo(drag.line + (event.clientY - drag.y) * drag.base / (drag.travel || 1));
  });
  for (const name of ['pointerup','pointercancel','lostpointercapture']) bar.addEventListener(name, () => { finish(); reveal(); });
  bar.addEventListener('keydown', event => {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    const current = terminal.buffer.active.viewportY;
    const destinations = {ArrowDown:current+1, ArrowUp:current-1, PageDown:current+terminal.rows-1, PageUp:current-terminal.rows+1, Home:0, End:terminal.buffer.active.baseY};
    if (!(event.key in destinations)) return;
    event.preventDefault(); event.stopPropagation(); scrollTo(destinations[event.key]); reveal();
  });
  bar.addEventListener('wheel', event => {
    // Forward to xterm so mouse reporting and alternate scroll modes stay intact.
    if (event.ctrlKey) return;
    host.querySelector('.xterm').dispatchEvent(new WheelEvent('wheel', {deltaY:event.deltaY, deltaX:event.deltaX, deltaMode:event.deltaMode, bubbles:true, cancelable:true, shiftKey:event.shiftKey, altKey:event.altKey, metaKey:event.metaKey}));
    event.preventDefault();
  }, {passive:false});
  update();
}
