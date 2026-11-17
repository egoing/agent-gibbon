(() => {
  const invoke = window.__TAURI__.core.invoke;
  const status = document.getElementById('status');
  const report = error => { status.textContent = String(error); };
  const copy = document.getElementById('copy');
  const size = document.getElementById('font-size');
  copy.disabled = size.disabled = true;
  copy.onclick = () => invoke('web_chrome', { action: 'copy-url' }).catch(report);
  window.gibbonContentSize.bind('web', size,
    scale => invoke('web_chrome', { action: 'zoom', scale }), report);
  const apply = info => {
    copy.disabled = size.disabled = info.welcome;
    document.getElementById('title').textContent = info.url || '웹';
    document.getElementById('pin').setAttribute('aria-pressed', String(info.always_on_top));
    document.querySelector('main').hidden = !info.welcome;
  };
  window.gibbonWebChrome = { apply };
  document.getElementById('pin').onclick = () => invoke('web_chrome', { action: 'pin' }).then(apply).catch(report);
  document.getElementById('close').onclick = () => invoke('web_chrome', { action: 'close' }).catch(report);
  document.querySelectorAll('.resize').forEach(handle => handle.addEventListener('pointerdown', event => {
    if (event.button !== 0) return;
    event.preventDefault();
    invoke('web_chrome', { action: `resize-${handle.dataset.direction}` }).catch(report);
  }));
  const initialize = () => invoke('web_chrome', { action: 'inspect' }).then(apply);
  if (window.gibbonRegionFrame) window.gibbonRegionFrame.prepare(initialize);
  else initialize().catch(report);
})();
