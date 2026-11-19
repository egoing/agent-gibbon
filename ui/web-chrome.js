(() => {
  const invoke = window.__TAURI__.core.invoke;
  const status = document.getElementById('status');
  const report = error => { status.textContent = String(error); };
  const copy = document.getElementById('copy');
  const size = document.getElementById('font-size');
  const back = document.getElementById('web-back');
  const reload = document.getElementById('web-reload');
  let reloading = false;
  reload.onclick = async () => {
    if (reload.disabled || reloading) return;
    reloading = true;
    try { await invoke('web_chrome', { action: 'reload' }); }
    catch (error) { report(error); }
    finally { reloading = false; }
  };
  back.onclick = () => invoke('web_chrome', { action: 'back' }).catch(report);
  copy.disabled = size.disabled = back.disabled = reload.disabled = true;
  copy.onclick = () => invoke('web_chrome', { action: 'copy-url' }).catch(report);
  window.jajanContentSize.bind('web', size,
    scale => invoke('web_chrome', { action: 'zoom', scale }), report);
  const apply = info => {
    copy.disabled = size.disabled = back.disabled = reload.disabled = info.welcome;
    document.getElementById('title').textContent = info.url || '웹';
    document.getElementById('pin').setAttribute('aria-pressed', String(info.always_on_top));
    document.querySelector('main').hidden = !info.welcome;
  };
  window.jajanWebChrome = { apply };
  document.getElementById('pin').onclick = () => invoke('web_chrome', { action: 'pin' }).then(apply).catch(report);
  document.getElementById('close').onclick = () => invoke('web_chrome', { action: 'close' }).catch(report);
  document.querySelectorAll('.resize').forEach(handle => handle.addEventListener('pointerdown', event => {
    if (event.button !== 0) return;
    event.preventDefault();
    invoke('web_chrome', { action: `resize-${handle.dataset.direction}` }).catch(report);
  }));
  const initialize = () => invoke('web_chrome', { action: 'inspect' }).then(apply);
  if (window.jajanRegionFrame) window.jajanRegionFrame.prepare(initialize);
  else initialize().catch(report);
})();
