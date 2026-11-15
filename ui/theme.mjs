export const colorPalette = [
  {name:'라임', background:'#bdd657', foreground:'#20271b'},
  {name:'코랄', background:'#db704f', foreground:'#291b16'},
  {name:'민트', background:'#6baa92', foreground:'#172a23'},
  {name:'아이보리', background:'#e5e4d2', foreground:'#27291f'},
  {name:'차콜', background:'#4d4d4b', foreground:'#e5e4d2'},
];
export function contrastRatio(a, b) {
  const lum = hex => hex.slice(1).match(/../g).map(v => parseInt(v,16)/255)
    .map(v => v <= .04045 ? v/12.92 : ((v+.055)/1.055)**2.4)
    .reduce((sum,v,i) => sum+v*[.2126,.7152,.0722][i],0);
  const x=lum(a), y=lum(b);
  return (Math.max(x,y)+.05)/(Math.min(x,y)+.05);
}

// Unknown HEX colors use the higher-contrast black/white pair. Fixed candidates
// use their reference-inspired foreground; no alpha is applied to readable text.
export function themeFor(color) {
  const channels = hex => hex.slice(1).match(/../g).map(v => parseInt(v,16));
  const mix = (a,b,weight) => '#' + channels(a).map((v,i) =>
    Math.round(v*(1-weight)+channels(b)[i]*weight).toString(16).padStart(2,'0')).join('');
  const light = contrastRatio(color,'#000000') >= contrastRatio(color,'#ffffff');
  const foreground = colorPalette.find(p => p.background === color.toLowerCase())?.foreground || (light ? '#000000' : '#ffffff');
  const panel = mix(color,light ? '#ffffff' : '#000000',.1);
  const readableMix = (minimum,start) => {
    for(let weight=start;weight<=1.001;weight+=.01) {
      const candidate=mix(color,foreground,weight);
      if([color,panel].every(bg=>contrastRatio(candidate,bg)>=minimum)) return candidate;
    }
    return foreground;
  };
  const errorCandidate = light ? '#9c2424' : '#ffb9b9';
  return {
    foreground,
    secondary:readableMix(4.5,.55),
    border:readableMix(3,.3),
    panel,
    error:contrastRatio(color,errorCandidate)>=4.5 ? errorCandidate : foreground,
    scheme:light ? 'light' : 'dark',
  };
}

// Independent memo designs. Stored per-memo colors are used only for color.
const memoThemes = {
  white: { background: '#f8f8f8', foreground: '#2d2d2d', border: '#d6d6d6', panel: '#ffffff', error: '#8c2222', scheme: 'light' },
  gray: { background: '#dfe2dd', foreground: '#2c2f7e', border: '#a0a3bc', panel: '#cdd0d3', error: '#8c2222', scheme: 'light' },
  black: { background: '#262627', foreground: '#dfe2dd', border: '#444444', panel: '#393938', error: '#ffb4ab', scheme: 'dark' },
};
export function memoThemeFor(color, {design = 'gray', systemDark = false} = {}) {
  const resolved = design === 'system' ? (systemDark ? 'black' : 'gray') : design;
  return design === 'color' ? {...themeFor(color), background: color} : {...(memoThemes[resolved] || memoThemes.gray)};
}
