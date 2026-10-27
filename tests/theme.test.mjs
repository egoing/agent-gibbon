import test from 'node:test';
import assert from 'node:assert/strict';
import { themeFor } from '../ui/theme.mjs';

test('light and dark backgrounds have readable foregrounds', () => {
  assert.equal(themeFor('#ffffff').foreground, '#000000');
  assert.equal(themeFor('#ffffff').scheme, 'light');
  assert.equal(themeFor('#000000').foreground, '#ffffff');
  assert.equal(themeFor('#000000').scheme, 'dark');
  assert.equal(themeFor('#20252e').scheme, 'dark');
});

test('sampled RGB space has at least 4.5:1 text contrast', () => {
  for (let r = 0; r <= 255; r += 17) {
    for (let g = 0; g <= 255; g += 17) {
      for (let b = 0; b <= 255; b += 17) {
        const hex = '#' + [r, g, b].map(c => c.toString(16).padStart(2, '0')).join('');
        const theme = themeFor(hex);
        for (const background of [hex, theme.panel]) {
          const values = background.slice(1).match(/../g).map(c => parseInt(c, 16));
          const linear = values.map(c => c / 255).map(c => c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
          const luminance = linear.reduce((sum, c, i) => sum + c * [0.2126, 0.7152, 0.0722][i], 0);
          const text = themeFor(hex).foreground === '#000000' ? 0 : 1;
          const contrast = (Math.max(luminance, text) + 0.05) / (Math.min(luminance, text) + 0.05);
          assert.ok(contrast >= 4.5, `${hex} on ${background}: ${contrast}`);
        }
      }
    }
  }
});

test('error text retains contrast on saturated memo colors', () => {
  const luminance = hex => hex.slice(1).match(/../g).map(c => parseInt(c, 16) / 255)
    .map(c => c <= .04045 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4)
    .reduce((sum, c, i) => sum + c * [.2126, .7152, .0722][i], 0);
  for (const background of ['#ff0000', '#00ff00', '#0000ff', '#fff5cf', '#20252e', '#777777']) {
    const text = luminance(themeFor(background).error), surface = luminance(background);
    assert.ok((Math.max(text, surface) + .05) / (Math.min(text, surface) + .05) >= 4.5);
  }
});
