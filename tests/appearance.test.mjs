import test from 'node:test';
import assert from 'node:assert/strict';
import {readMemoAppearance,memoDesignKey} from '../ui/appearance.mjs';
import {memoThemeFor,themeFor} from '../ui/theme.mjs';
test('memo defaults to gray regardless of manager selection, old color mode or OS appearance',()=>{
 const data=new Map([['agent-gibbon.manager-theme','black'],['agent-gibbon.memo-color-mode','on']]);
 const env={localStorage:{getItem:k=>data.get(k)},matchMedia:()=>({matches:true})};
 assert.deepEqual(readMemoAppearance(env),{design:'gray'});
 for(const design of ['white','gray','black','color']){data.set(memoDesignKey,design);assert.equal(readMemoAppearance(env).design,design);}
 data.set(memoDesignKey,'invalid');assert.equal(readMemoAppearance(env).design,'gray');
});
test('inaccessible memo preferences fall back to gray',()=>{
 assert.deepEqual(readMemoAppearance({get localStorage(){throw Error('blocked');}}),{design:'gray'});
});
test('independent theme roles and color mode display preserve the individual color across cycles',()=>{
 const colors=['#ffeeaa','#ccddff','#123456'];
 const expected={white:['#f8f8f8','#2d2d2d','#d6d6d6','#ffffff'],gray:['#dfe2dd','#2c2f7e','#a0a3bc','#cdd0d3'],black:['#262627','#dfe2dd','#444444','#393938']};
 for(const [design,roles] of Object.entries(expected))for(const color of colors){
  const theme=memoThemeFor(color,{design});assert.deepEqual([theme.background,theme.foreground,theme.border,theme.panel],roles);
  assert.deepEqual(memoThemeFor(color,{design:'color'}),{...themeFor(color),background:color});
  assert.deepEqual(memoThemeFor(color,{design}),theme);
 }
});

test('only the independent system memo choice follows day/night appearance',()=>{
 const media={matches:false};const data=new Map([[memoDesignKey,'system'],['agent-gibbon.manager-theme','white']]);const env={localStorage:{getItem:k=>data.get(k)},matchMedia:()=>media};
 assert.equal(memoThemeFor('#ffeeaa',readMemoAppearance(env)).background,'#dfe2dd');media.matches=true;assert.equal(memoThemeFor('#ffeeaa',readMemoAppearance(env)).background,'#262627');
 data.set('agent-gibbon.manager-theme','gray');assert.equal(memoThemeFor('#ffeeaa',readMemoAppearance(env)).background,'#262627');
 data.set(memoDesignKey,'gray');assert.equal(memoThemeFor('#ffeeaa',readMemoAppearance(env)).background,'#dfe2dd');
});
