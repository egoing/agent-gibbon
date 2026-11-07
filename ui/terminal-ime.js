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
    // xterm renders Hangul in two terminal cells. Natural browser text is
    // narrower with the fallback font, so size each provisional glyph using
    // the same screen grid instead of letting the browser pack the word.
    const screen = host.querySelector('.xterm-screen');
    const cellWidth = screen.getBoundingClientRect().width / terminal.cols;
    const cellHeight = screen.getBoundingClientRect().height / terminal.rows;
    const characters = Array.from(pending);
    preview.replaceChildren(...characters.map((character, index) => {
      const cell = document.createElement('span');
      cell.className = 'gibbon-ime-cell' + (index === characters.length - 1 ? ' gibbon-ime-current' : '');
      cell.textContent = character;
      // Match xterm's default Unicode 6 widths for the Hangul ranges handled
      // by this adapter (modern syllables/compatibility jamo are two cells).
      const columns = /[\u1100-\u115f\u3130-\u318f\uac00-\ud7a3]/.test(character) ? 2
        : /[\u1160-\u11ff]/.test(character) ? 0 : 1;
      cell.style.width = `${columns * cellWidth}px`;
      return cell;
    }));
    preview.classList.toggle('gibbon-ime-fallback', !!pending);
    preview.classList.toggle('active', !!pending);
    const row = terminal.buffer.active.cursorY;
    Object.assign(preview.style, {
      left: `${terminal.buffer.active.cursorX * cellWidth}px`,
      top: `${row * cellHeight}px`,
      fontFamily: terminal.options.fontFamily, fontSize: `${terminal.options.fontSize}px`,
      lineHeight: `${cellHeight}px`
    });
  }
  // Font-size changes, resize and asynchronous PTY output can update the grid
  // while a word is pending. Re-anchor without committing or moving focus.
  terminal.onRender?.(() => { if (pending && !native) show(); });
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
