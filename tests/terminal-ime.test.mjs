import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('../ui/terminal-ime.js',import.meta.url),'utf8');
function fixture(userAgent='AppleWebKit/605.1.15') {
  function element() {
    const classes=new Set(), listeners=new Map();
    return {style:{},value:'',textContent:'',children:[],className:'',
      classList:{toggle(name,on){on?classes.add(name):classes.delete(name);},contains:name=>classes.has(name)},
      replaceChildren(...nodes){this.children=nodes;},
      addEventListener(type,fn){listeners.set(type,fn);},
      dispatch(type,props={}) {const e={target:textarea,stopImmediatePropagation(){this.stopped=true;},...props};listeners.get(type)?.(e);return e;}
    };
  }
  const textarea=element(), preview=element(), host=element(), sent=[];
  const screen={getBoundingClientRect:()=>({width:800,height:400})};
  host.querySelector=selector=>selector==='.composition-view'?preview:screen;
  let render;
  const terminal={onRender(fn){render=fn;},textarea,element:host,cols:80,rows:20,buffer:{active:{cursorX:4,cursorY:2}},options:{fontFamily:'Menlo',fontSize:13}};
  const context={window:{},navigator:{userAgent},document:{createElement:element,createTextNode:textContent=>({textContent})}};
  vm.runInNewContext(source,context);context.window.installTerminalIme(terminal,s=>sent.push(s));
  const input=(value,data=value,inputType='insertReplacementText')=>{textarea.value=value;return host.dispatch('input',{data,inputType});};
  const key=(key,props={})=>host.dispatch('keydown',{key,keyCode:0,...props});
  const text=()=>preview.children.map(n=>n.textContent).join('');
  const current=()=>preview.children.find(n=>n.className.includes('gibbon-ime-current'))?.textContent;
  return {textarea,preview,host,sent,input,key,text,current,terminal,render:()=>render()};
}
test('WebKit word stays editable while only the current Hangul character is highlighted',()=>{
  const f=fixture();
  for(const value of ['ㅎ','하','한','한ㄱ','한그','한글']) {
    assert.equal(f.input(value).stopped,true);
    assert.equal(f.text(),value);
    assert.equal(f.current(),Array.from(value).at(-1));
    assert.equal(f.preview.classList.contains('gibbon-ime-fallback'),true);
    assert.deepEqual(f.sent,[]); // Do not send jamo or a mutable final consonant.
  }
  f.input('한글\u00a0','\u00a0');
  assert.deepEqual(f.sent,['한글 ']);
  assert.equal(f.preview.classList.contains('active'),false);
  assert.equal(f.preview.classList.contains('gibbon-ime-fallback'),false);
  assert.equal(f.text(),'');
});
test('final consonant migration and deletion update the highlighted suffix without PTY edits',()=>{
  const f=fixture();
  for(const value of ['ㅎ','하','한','하나','하낭','하나','하ㄴ','하','ㅎ','']) f.input(value);
  assert.equal(f.preview.classList.contains('active'),false);
  assert.deepEqual(f.sent,[]);
  f.input('한글');f.input('한');assert.equal(f.current(),'한');
  f.key('ArrowLeft');assert.deepEqual(f.sent,['한']);assert.equal(f.text(),'');
});
test('commit boundaries remove the fallback background before newline or navigation',()=>{
  for(const key of ['Enter','ArrowLeft','ArrowRight','Delete','Tab']) {
    const f=fixture();f.input('한글');f.key(key);
    assert.deepEqual(f.sent,['한글']);assert.equal(f.preview.classList.contains('active'),false);
  }
  for(const boundary of [' ','.','!']) {
    const f=fixture();f.input('한글');f.input('한글'+boundary,boundary);
    assert.deepEqual(f.sent,['한글'+boundary]);assert.equal(f.preview.classList.contains('active'),false);
  }
});
test('cancel, blur and paste preserve existing commit/cancel semantics',()=>{
  for(const [key,props] of [['Escape',{}],['c',{ctrlKey:true}]]) {
    const f=fixture();f.input('한글');f.key(key,props);assert.deepEqual(f.sent,[]);assert.equal(f.text(),'');
  }
  for(const type of ['blur','paste']) {
    const f=fixture();f.input('한글');(type==='blur'?f.textarea:f.host).dispatch(type);
    assert.deepEqual(f.sent,['한글']);assert.equal(f.preview.classList.contains('gibbon-ime-fallback'),false);
  }
});
test('native composition and ordinary English/paste input remain owned by xterm',()=>{
  const f=fixture();
  f.host.dispatch('compositionstart');
  assert.equal(f.input('한','한','insertCompositionText').stopped,undefined);
  assert.equal(f.preview.classList.contains('gibbon-ime-fallback'),false);
  f.host.dispatch('compositionend');
  assert.equal(f.input('abc','abc','insertText').stopped,undefined);
  assert.equal(f.input('한글','한글','insertFromPaste').stopped,undefined);
  assert.deepEqual(f.sent,[]);
});
test('switching to native composition clears fallback styling; Chromium is untouched',()=>{
  const f=fixture();f.input('한');f.host.dispatch('compositionstart');
  assert.deepEqual(f.sent,['한']);assert.equal(f.preview.classList.contains('gibbon-ime-fallback'),false);
  const chrome=fixture('AppleWebKit/537.36 Chrome/130.0');
  assert.equal(chrome.input('한').stopped,undefined);assert.deepEqual(chrome.sent,[]);
});

test('pending Hangul and ASCII use terminal column widths, including after resize',()=>{
  const f=fixture();f.input('한a글');
  assert.deepEqual(f.preview.children.map(c=>c.style.width),['20px','10px','20px']);
  assert.equal(f.preview.style.left,'40px');assert.equal(f.preview.style.lineHeight,'20px');
  f.terminal.cols=100;f.terminal.options.fontSize=18;f.render();
  assert.deepEqual(f.preview.children.map(c=>c.style.width),['16px','8px','16px']);
  assert.equal(f.preview.style.left,'32px');assert.equal(f.preview.style.fontSize,'18px');
  assert.deepEqual(f.sent,[]);f.key('Enter');assert.deepEqual(f.sent,['한a글']);
});
