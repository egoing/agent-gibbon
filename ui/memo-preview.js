import {readMemoAppearance} from './appearance.mjs';
import {memoThemeFor} from './theme.mjs';
// Fixed examples only: no saved memo, IPC or content lookup is involved.
const media = window.matchMedia('(prefers-color-scheme: dark)');
function render(design = readMemoAppearance(window).design) {
  for (const [id, color] of [['memo-preview-first', '#fee93e'], ['memo-preview-second', '#89f0ff']]) {
    const theme = memoThemeFor(color, {design, systemDark: media.matches});
    const sample = document.getElementById(id);
    for (const key of ['background', 'foreground', 'border']) sample.style.setProperty(`--preview-${key}`, theme[key]);
    sample.style.colorScheme = theme.scheme;
  }
}
window.gibbonMemoPreview = render;
window.addEventListener('storage', () => render());
media.addEventListener('change', () => render());
render();
