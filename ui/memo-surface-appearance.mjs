import {memoThemeFor} from './theme.mjs';
import {readMemoAppearance, memoDesignKey} from './appearance.mjs';

// Trusted Welcome surfaces and toolbars follow the independent memo design.
// External web content and PTY output keep their own palettes.
export function installMemoSurfaceAppearance(env = window, root = document.documentElement) {
  let color = '#20252e';
  const apply = () => {
    const appearance = readMemoAppearance(env);
    const theme = memoThemeFor(color, appearance);
    if (root.dataset) root.dataset.memoDesign = appearance.design;
    for (const role of ['background', 'foreground', 'border', 'panel', 'error']) {
      root.style.setProperty(`--${role}`, theme[role]);
    }
    root.style.setProperty('--secondary', theme.secondary || theme.foreground);
    root.style.colorScheme = theme.scheme;
  };
  const setColor = value => {
    if (/^#[0-9a-f]{6}$/i.test(value || '')) { color = value; apply(); }
  };
  apply();
  // Only trusted chrome loads this module. No bridge is added to remote pages.
  env.__TAURI__?.event?.listen('memo-color', event => setColor(event.payload))?.catch(() => {});
  const invoke = env.__TAURI__?.core?.invoke;
  if (invoke) {
    const web = env.location?.pathname?.endsWith('/web.html');
    invoke(web ? 'web_chrome' : 'terminal_info', web ? {action:'inspect'} : undefined)
      .then(info => setColor(info.color)).catch(() => {});
  }
  env.addEventListener('storage', event => {
    if (event.key !== memoDesignKey && event.key !== null) return;
    try { if (event.storageArea !== env.localStorage) return; } catch { return; }
    apply();
  });
  env.matchMedia?.('(prefers-color-scheme: dark)').addEventListener('change', apply);
}
