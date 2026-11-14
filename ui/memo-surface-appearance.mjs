import {memoThemeFor} from './theme.mjs';
import {readMemoAppearance, memoDesignKey} from './appearance.mjs';

// Trusted Welcome surfaces and toolbars follow the independent memo design.
// External web content and PTY output keep their own palettes.
export function installMemoSurfaceAppearance(env = window, root = document.documentElement) {
  const apply = () => {
    const theme = memoThemeFor('#20252e', readMemoAppearance(env));
    for (const role of ['background', 'foreground', 'border', 'panel']) {
      root.style.setProperty(`--${role}`, theme[role]);
    }
    root.style.colorScheme = theme.scheme;
  };
  apply();
  env.addEventListener('storage', event => {
    if (event.key !== memoDesignKey && event.key !== null) return;
    try { if (event.storageArea !== env.localStorage) return; } catch { return; }
    apply();
  });
  env.matchMedia?.('(prefers-color-scheme: dark)').addEventListener('change', apply);
}
