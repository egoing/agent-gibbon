// Reproducible palette contrast/distance and explanatory SVG; no runtime sampling.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {colorPalette,themeFor,contrastRatio} from '../ui/theme.mjs';
function lab(hex){
 const [r,g,b]=hex.slice(1).match(/../g).map(v=>parseInt(v,16)/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4);
 const l=Math.cbrt(.4122214708*r+.5363325363*g+.0514459929*b),m=Math.cbrt(.2119034982*r+.6806995451*g+.1073969566*b),s=Math.cbrt(.0883024619*r+.2817188376*g+.6299787005*b);
 return [.2104542553*l+.793617785*m-.0040720468*s,1.9779984951*l-2.428592205*m+.4505937099*s,.0259040371*l+.7827717662*m-.808675766*s];
}
const rows=colorPalette.map(p=>({...p,...themeFor(p.background),primaryContrast:contrastRatio(p.background,p.foreground),secondaryContrast:contrastRatio(p.background,themeFor(p.background).secondary)}));
const distances=[];for(let i=0;i<rows.length;i++)for(let j=i+1;j<rows.length;j++)distances.push({pair:[rows[i].name,rows[j].name],deltaOK:100*Math.hypot(...lab(rows[i].background).map((v,k)=>v-lab(rows[j].background)[k]))});
distances.sort((a,b)=>a.deltaOK-b.deltaOK);
assert(distances[0].deltaOK >= 15, 'Fixed candidates must remain distinguishable under the chosen ΔOK >= 15 policy');
const out='docs/verification/color-palette';fs.mkdirSync(out,{recursive:true});
fs.writeFileSync(out+'/metrics.json',JSON.stringify({method:'100 × Euclidean OKLab (D65 sRGB)',minimum:distances[0],rows,distances},null,2)+'\n');
const cards=rows.map((p,i)=>{const x=24+i%3*330,y=80+Math.floor(i/3)*250;return `<g transform="translate(${x} ${y})"><rect width="310" height="220" rx="4" fill="${p.background}"/><g fill="${p.foreground}" font-family="Arial, sans-serif"><text x="12" y="19" font-size="12">${p.name}　${p.background}</text><text x="256" y="19" font-size="13">aA　×</text><text x="12" y="61" font-size="19">작은 메모, 선명한 글자</text><text x="12" y="88" font-size="13">본문 · 링크 · 도구 아이콘</text><text x="12" y="119" font-size="12" fill="${p.secondary}">보조 문구 · ${p.secondary}</text><path d="M12 148H298" stroke="${p.foreground}" opacity=".25"/><text x="12" y="171" font-size="12" fill="${p.secondary}">저장됨 · Feedback</text><text x="12" y="199" font-size="11">본문 ${p.primaryContrast.toFixed(2)}:1 · 보조 ${p.secondaryContrast.toFixed(2)}:1</text></g></g>`;}).join('');
fs.writeFileSync('docs/design/color-palette.svg',`<svg xmlns="http://www.w3.org/2000/svg" width="1038" height="625" viewBox="0 0 1038 625"><rect width="1038" height="625" fill="#f3f3f3"/><g font-family="Arial,sans-serif" fill="#282828"><text x="24" y="30" font-size="18">컬러 모드 · 참고 이미지 대표색 5개</text><text x="24" y="55" font-size="12">설명용 시안 · 최소 ΔOK ${distances[0].deltaOK.toFixed(2)} (${distances[0].pair.join(' / ')}) · 실제 앱 렌더는 별도 검증</text>${cards}<text x="24" y="600" font-size="12">텍스트·캡처 프레임·터미널/웹 Toolbar와 Welcome에 적용 · 원본 이미지/ANSI/외부 페이지 보존</text></g></svg>`);
console.log(JSON.stringify({minimum:distances[0],rows},null,2));
