document.addEventListener('pointerdown', (event) => {
  if (event.button !== 0 || !document.documentElement.hasAttribute('data-visible')) return;
  window.jajanInvoke('overview_action', {
    action: 'clearfocus',
    generation: window.overviewGeneration,
  }).catch(console.error);
});
