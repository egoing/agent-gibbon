// WebKit may replace Hangul in the textarea without composition events.
// Keep that provisional text local; never send partial jamo to the PTY.
// Native composition events continue to be owned by xterm.
window.installTerminalIme = function (terminal, send) {
  const textarea = terminal.textarea;
  const host = terminal.element;
  if (!textarea || !host || !/AppleWebKit/.test(navigator.userAgent) || /Chrome|Chromium/.test(navigator.userAgent)) return;
  const hangul = /[\u1100-\u11ff\u3130-\u318f\ua960-\ua97f\uac00-\ud7af\ud7b0-\ud7ff]/;
  let pending = '', native = false;
  const preview = host.querySelector('.composition-view');
  function show() {
    if (!preview) return;
    preview.textContent = pending;
    preview.classList.toggle('active', !!pending);
    const screen = host.querySelector('.xterm-screen');
    const row = terminal.buffer.active.cursorY;
    Object.assign(preview.style, {
      left: `${terminal.buffer.active.cursorX * screen.clientWidth / terminal.cols}px`,
      top: `${row * screen.clientHeight / terminal.rows}px`,
      fontFamily: terminal.options.fontFamily, fontSize: `${terminal.options.fontSize}px`
    });
  }
  function finish(cancel = false) {
    if (!pending) return;
    const value = pending; pending = ''; textarea.value = ''; show();
    if (!cancel) send(value.replaceAll('\u00a0', ' '));
  }
  host.addEventListener('compositionstart', () => { finish(); native = true; }, true);
  host.addEventListener('compositionend', () => { native = false; }, true);
  host.addEventListener('input', e => {
    if (e.target !== textarea || native || e.isComposing) return;
    if (!pending && !(hangul.test(e.data || '') && ['insertText', 'insertReplacementText'].includes(e.inputType))) return;
    e.stopImmediatePropagation();
    pending = textarea.value;
    show();
    // Space/punctuation commits the word. Until then WebKit retains the whole
    // editable word, including final consonants which can move to next syllable.
    if (pending && !hangul.test(pending.slice(-1))) finish();
  }, true);
  host.addEventListener('keydown', e => {
    if (e.target !== textarea || native) return;
    if (e.keyCode === 229) { e.stopImmediatePropagation(); return; }
    if (!pending) return;
    if (['Shift','Control','Alt','Meta','CapsLock'].includes(e.key)) return;
    if (e.key === 'Backspace' || (!e.ctrlKey && !e.metaKey && e.key.length === 1 && e.key !== ' ')) {
      e.stopImmediatePropagation(); return; // Browser edits the pending textarea.
    }
    finish(e.key === 'Escape' || (e.ctrlKey && e.key.toLowerCase() === 'c'));
  }, true);
  host.addEventListener('keypress', e => { if (pending && !native) e.stopImmediatePropagation(); }, true);
  textarea.addEventListener('blur', () => finish());
  host.addEventListener('paste', () => finish(), true);
};
