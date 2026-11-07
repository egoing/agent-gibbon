import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {memoThemeFor} from '../ui/theme.mjs';
const css=readFileSync(new URL('../ui/terminal.css',import.meta.url),'utf8');
const rules=[...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map(([,selectors,body])=>({selectors:selectors.trim().split(',').map(s=>s.trim()),style:Object.fromEntries(body.split(';').map(s=>s.trim().split(/:(.*)/s)).filter(a=>a.length>1).map(([k,v])=>[k.trim(),v.trim()]))}));
function style(selector){return Object.assign({},...rules.filter(r=>r.selectors.includes(selector)).map(r=>r.style));}
function resolve(value,theme){if(!value||value==='inherit'||value==='currentColor')return theme.foreground;return value.replace(/var\(--(\w+)\)/g,(_,role)=>theme[role]);}
function luminance(color){assert.match(color,/^#[0-9a-f]{6}$/i);const c=color.slice(1).match(/../g).map(v=>parseInt(v,16)/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4);return c[0]*.2126+c[1]*.7152+c[2]*.0722;}
function contrast(a,b){const x=luminance(a),y=luminance(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05);}
const cases=[...['white','gray','black'].map(design=>[design,memoThemeFor('#20252e',{design})]),...[false,true].map(systemDark=>['system-'+systemDark,memoThemeFor('#20252e',{design:'system',systemDark})]),...['#ffeeaa','#123456','#ff0000','#00ff00','#0000ff','#777777','#ffffff','#000000'].map(color=>[color,memoThemeFor(color,{design:'color'})])];
for(const [name,theme] of cases)test(`Welcome text and controls retain contrast in ${name}`,()=>{
 const checks=[['#welcome .plain-row:hover',theme.foreground],['#welcome .select-row[aria-pressed="true"]',theme.foreground],['#run-task',theme.foreground],['#command-preview',theme.foreground],['#rename-title',theme.foreground],['.folder-parent',null],['#home-hint',null],['#title-details summary',null]];
 for(const [selector,text] of checks){const s=style(selector);const bg=resolve(s.background||'var(--background)',theme);const fg=resolve(text||s.color,theme);assert.ok(contrast(fg,bg)>=4.5,`${selector}: ${fg} on ${bg} = ${contrast(fg,bg).toFixed(2)}:1`);}
});
