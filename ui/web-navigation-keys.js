// The host supplies a navigation callback. Remote content uses browser APIs only.
function installWebNavigationKeys(navigate) {
  const mac = /Mac/.test(navigator.platform);
  window.addEventListener('keydown', event => {
    if (!event.isTrusted || event.isComposing || event.shiftKey || event.defaultPrevented) return;
    const command = mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
    let action;
    if ((!event.altKey && command && event.key.toLowerCase() === 'r') ||
        (event.key === 'F5' && !event.altKey && !event.ctrlKey && !event.metaKey)) action = 'reload';
    if (mac && command && !event.altKey && ['[', ']'].includes(event.key))
      action = event.key === '[' ? 'back' : 'forward';
    if (event.altKey && !event.metaKey && !event.ctrlKey && ['ArrowLeft', 'ArrowRight'].includes(event.key)) {
      if (mac && event.composedPath().some(el => el.isContentEditable || /^(INPUT|TEXTAREA)$/.test(el.tagName))) return;
      action = event.key === 'ArrowLeft' ? 'back' : 'forward';
    }
    if (action && navigate(action) !== false) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
  }, true);
}
