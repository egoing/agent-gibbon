(() => {
  const invoke = window.__TAURI__.core.invoke;
  const status = document.getElementById('status');
  const report = error => { status.textContent = String(error); };
  const apply = info => {
    document.getElementById('title').textContent = info.url || '웹';
    document.getElementById('pin').setAttribute('aria-pressed', String(info.always_on_top));
    document.querySelector('main').hidden = !info.welcome;
  };
  document.getElementById('pin').onclick = () => invoke('web_chrome', { action: 'pin' }).then(apply).catch(report);
  document.getElementById('close').onclick = () => invoke('web_chrome', { action: 'close' }).catch(report);
  document.getElementById('controls').addEventListener('pointerdown', event => {
    if (event.button !== 0 || event.target.closest('button')) return;
    invoke('web_chrome', { action: 'drag' }).catch(report);
  });
  document.querySelectorAll('.resize').forEach(handle => handle.addEventListener('pointerdown', event => {
    if (event.button !== 0) return;
    event.preventDefault();
    invoke('web_chrome', { action: `resize-${handle.dataset.direction}` }).catch(report);
  }));
  invoke('web_chrome', { action: 'inspect' }).then(apply).catch(report);
})();
