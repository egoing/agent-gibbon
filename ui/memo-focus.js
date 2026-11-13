// The native memo and its visible Feedback companion share one focus frame.
const memoWindow = window.__TAURI__.window.getCurrentWindow();
let focusRevision = 0;
const applyMemoFocus = focused => document.body.classList.toggle('memo-focused', focused);
await window.__TAURI__.event.listen('memo-frame-focus', ({ payload }) => {
  focusRevision++;
  applyMemoFocus(payload);
}, { target: { kind: 'Window', label: memoWindow.label } });
const initialFocusRevision = focusRevision;
const initiallyFocused = await window.__TAURI__.core.invoke('memo_frame_focus');
if (focusRevision === initialFocusRevision) applyMemoFocus(initiallyFocused);
