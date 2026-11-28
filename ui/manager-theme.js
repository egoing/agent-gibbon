'use strict';
// Loaded synchronously in the manager head so the first paint uses the preference.
// Only a fixed theme name crosses local IPC; memo content is never read.
(() => {
  const permissionView = document.documentElement.dataset.themeView === 'permission';
  const key = 'jajan.manager-theme';
  const memoDesignKey = 'jajan.memo-design';
  const validDesign = value => ['white', 'gray', 'black', 'system', 'color'].includes(value);
  let memoDesign = 'gray';
  try { const value = window.localStorage.getItem(memoDesignKey); if (validDesign(value)) memoDesign = value; } catch {}
  const valid = value => ['white', 'gray', 'black'].includes(value);
  const media = window.matchMedia('(prefers-color-scheme: dark)');
  let selected = null;
  let nativeQueue = Promise.resolve(), nativeRequested = null, nativeError = false;
  function renderNativeStatus() {
    if (document.readyState === 'loading') return;
    const status = document.getElementById('theme-native-status');
    if (status) status.hidden = !nativeError;
  }
  function syncNative(theme) {
    const invoke = window.__TAURI__?.core?.invoke;
    if (!invoke || nativeRequested === theme) return;
    nativeRequested = theme;
    // Serialize to prevent a slow previous color overwriting the latest choice.
    nativeQueue = nativeQueue.then(() => invoke('manager_set_theme', {theme})).then(() => {
      if (nativeRequested === theme) { nativeError = false; renderNativeStatus(); }
    }).catch(() => {
      if (nativeRequested === theme) {
        nativeRequested = null; nativeError = true; renderNativeStatus();
      }
    });
  }
  try { const value = window.localStorage.getItem(key); if (valid(value)) selected = value; } catch {}
  const current = () => selected || (media.matches ? 'black' : 'gray');
  function apply() {
    document.documentElement.dataset.managerTheme = current();
    syncNative(current());
    if (permissionView || document.readyState === 'loading') return;
    for (const radio of document.querySelectorAll('input[name="manager-theme"]')) radio.checked = radio.value === (selected || 'system');
    renderNativeStatus();
    for (const radio of document.querySelectorAll('input[name="memo-design"]')) radio.checked = radio.value === memoDesign;
    window.jajanLanguage.bind(document.getElementById('theme-mode'), 'textContent', selected ? '선택한 테마를 사용 중입니다.' : '시스템 테마를 따르는 중입니다.');
    window.jajanMemoPreview?.(memoDesign);
  }
  function choose(value) {
    const result = document.getElementById('theme-result');
    try {
      if (value === null) window.localStorage.removeItem(key);
      else window.localStorage.setItem(key, value);
      selected = value;
      result.dataset.error = 'false';
      window.jajanLanguage.bind(result, 'textContent', value === null ? '시스템 테마를 따릅니다.' : '테마를 저장했습니다.');
    } catch {
      result.dataset.error = 'true';
      window.jajanLanguage.bind(result, 'textContent', '테마를 저장하지 못했습니다. 기존 설정을 유지합니다. 다시 시도하세요.');
    }
    apply();
  }
  function chooseMemoDesign(value) {
    const result = document.getElementById('theme-result');
    try {
      window.localStorage.setItem(memoDesignKey, value);
      memoDesign = value;
      result.dataset.error = 'false';
      window.jajanLanguage.bind(result, 'textContent', '메모 창 디자인을 저장했습니다.');
    } catch {
      result.dataset.error = 'true';
      window.jajanLanguage.bind(result, 'textContent', '메모 창 디자인을 저장하지 못했습니다. 기존 설정을 유지합니다. 다시 시도하세요.');
    }
    apply();
  }
  media.addEventListener('change', () => { if (selected === null) apply(); });
  window.addEventListener('storage', event => {
    if (event.key !== key && event.key !== memoDesignKey && event.key !== null) return;
    if (event.storageArea !== window.localStorage) return;
    if (event.key === key || event.key === null) selected = valid(event.newValue) ? event.newValue : null;
    if (event.key === memoDesignKey || event.key === null) memoDesign = validDesign(event.newValue) ? event.newValue : 'gray';
    apply();
  });
  apply();
  if (permissionView) return;
  document.addEventListener('DOMContentLoaded', () => {
    for (const radio of document.querySelectorAll('input[name="manager-theme"]')) {
      radio.addEventListener('click', () => { if (radio.checked && (valid(radio.value) || radio.value === 'system')) choose(radio.value === 'system' ? null : radio.value); });
    }
    for (const radio of document.querySelectorAll('input[name="memo-design"]')) {
      radio.addEventListener('click', () => { if (radio.checked && validDesign(radio.value)) chooseMemoDesign(radio.value); });
    }
    apply();
  });
})();
