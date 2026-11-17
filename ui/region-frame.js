// Only a newly drawn region receives a native placement ticket. A hidden
// WebView can suspend requestAnimationFrame, so prepare layout synchronously.
(() => {
  const invoke = window.__TAURI__.core.invoke;
  // An ordinary hidden WebView can load before onboarding allows app IPC.
  // Its regular refresh must still work after the user completes onboarding.
  // Keep a queued WebKit paint of the unthemed document transparent.
  // Set this in the head, before the parser creates the memo surface.
  const rootStyle = window.gibbonPreparingRegion && document.documentElement.style;
  if (rootStyle) rootStyle.opacity = '0';
  const revealContent = () => { if (rootStyle) rootStyle.removeProperty('opacity'); };
  const placement = invoke('region_frame_placement').catch(() => false);
  if (rootStyle) placement.then(pending => { if (!pending) revealContent(); });
  const initialContent = [];
  let showing;
  window.gibbonRegionFrame = {
    placement,
    prepare(work) {
      // A failed Welcome request is also a drawable state. Settle the task
      // after putting the error into the trusted page's existing status area.
      const prepared = placement.then(work).catch(error => {
        document.getElementById('status').textContent = String(error);
      });
      initialContent.push(prepared);
      return prepared;
    },
    async ready() {
      if (!await placement) return;
      if (!showing) {
        showing = Promise.all(initialContent).then(() => {
          revealContent();
          document.body.getBoundingClientRect();
          return invoke('region_frame_ready');
        }).catch(error => {
          showing = undefined;
          throw error;
        });
      }
      return showing;
    },
  };
})();
