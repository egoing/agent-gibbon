(() => {
  const invoke = window.gibbonInvoke || window.__TAURI__.core.invoke;
  const pin = document.getElementById('pin');
  const close = document.getElementById('close');
  const report = error => { close.title = `웹 세션 처리 실패: ${String(error)}`; };
  invoke('web_toolbar_info').then(info => {
    pin.setAttribute('aria-pressed', String(info.always_on_top));
  }).catch(report);
  pin.addEventListener('click', async () => {
    try {
      const pinned = await invoke('web_toggle_pin');
      pin.setAttribute('aria-pressed', String(pinned));
    } catch (error) {
      report(error);
    }
  });
  close.addEventListener('click', () => invoke('web_close').catch(report));
})();
