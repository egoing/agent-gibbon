document.addEventListener('pointerdown', (event) => {
  if (event.button !== 0 || !document.documentElement.hasAttribute('data-visible')) return;
  window.__TAURI__.core.invoke('overview_action', {
    action: 'clearfocus',
    generation: window.overviewGeneration,
  }).catch(console.error);
});
