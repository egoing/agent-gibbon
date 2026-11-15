// Fixed presentation only: no application commands, remote input, or bridge.
// Each top-level HTTP(S) document gets its own style; page CSS/CSP may override it.
(() => {
  if (!/^https?:$/.test(location.protocol)) return;
  const apply = () => {
    if (!document.head) return;
    const style = document.createElement('style');
    style.textContent = `
      :root { scrollbar-gutter: auto; }
      *::-webkit-scrollbar { width: 3px; height: 3px; }
      *::-webkit-scrollbar-track, *::-webkit-scrollbar-corner { background: transparent; }
      *::-webkit-scrollbar-thumb { background: rgba(96,96,96,.65); border-radius: 2px; }
      @media (prefers-color-scheme: dark) {
        *::-webkit-scrollbar-thumb { background: rgba(192,192,192,.65); }
      }
      @media (forced-colors: active) {
        *::-webkit-scrollbar { width: auto; height: auto; }
        *::-webkit-scrollbar-thumb { background: CanvasText; }
      }`;
    document.head.appendChild(style);
    // Keep the site's authored viewport. Desktop WebKit does not emulate a
    // touch device; this hint only fills the missing responsive viewport.
    if (!document.querySelector('meta[name="viewport" i]')) {
      const viewport = document.createElement('meta');
      viewport.name = 'viewport';
      viewport.content = 'width=device-width, initial-scale=1';
      document.head.appendChild(viewport);
    }
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', apply, {once: true});
  else apply();
})();
