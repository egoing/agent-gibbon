// WCAG relative luminance chooses the higher-contrast black/white foreground.
export function themeFor(color) {
  const channels = color.slice(1).match(/../g).map(value => parseInt(value, 16) / 255);
  const linear = channels.map(value => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
  const luminance = linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
  const blackContrast = (luminance + 0.05) / 0.05;
  const whiteContrast = 1.05 / (luminance + 0.05);
  const light = blackContrast >= whiteContrast;
  const mix = (target, weight) => '#' + channels.map(value =>
    Math.round((value * (1 - weight) + target * weight) * 255).toString(16).padStart(2, '0')
  ).join('');
  const foreground = light ? '#000000' : '#ffffff';
  const errorCandidate = light ? '#9c2424' : '#ffb9b9';
  const errorLinear = errorCandidate.slice(1).match(/../g).map(value => {
    const channel = parseInt(value, 16) / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  const errorLuminance = errorLinear[0] * .2126 + errorLinear[1] * .7152 + errorLinear[2] * .0722;
  const errorContrast = (Math.max(luminance, errorLuminance) + .05) / (Math.min(luminance, errorLuminance) + .05);
  return {
    error: errorContrast >= 4.5 ? errorCandidate : foreground,
    foreground: light ? '#000000' : '#ffffff',
    scheme: light ? 'light' : 'dark',
    border: mix(light ? 0 : 1, 0.3),
    panel: mix(light ? 1 : 0, 0.1),
  };
}
