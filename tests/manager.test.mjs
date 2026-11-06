import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
const source = readFileSync(new URL('../ui/manager.js', import.meta.url), 'utf8');
function setup() {
  const elements = new Map();
  const make = tagName => ({ tagName, hidden: false, children: [], dataset: {}, parentElement: { scrollTop: 0 }, setAttribute(k,v) { this[k] = v; }, append(x) { this.children.push(x); }, replaceChildren() { this.children = []; this.textContent = ''; }, addEventListener() {}, focus() {} });
  for (const id of ['notes-page','settings-page','notes-nav','settings-nav','retry','refresh','detail','list','error','settings-privacy','settings-skills','settings-privacy-nav','settings-skills-nav','settings-appearance','settings-appearance-nav']) elements.set(id, make());
  let data = [{id:'hidden',title:'숨긴 메모',html:'<h1>숨긴 메모</h1><p>내용</p>',capture:null,visible:false}];
  const calls = [], requests = [];
  let show = async ({id,visible}) => { data = data.map(n => n.id === id ? {...n,visible} : n); return visible; };
  let readNotes = () => data.map(n => ({...n}));
  const context = vm.createContext({document:{getElementById:id=>elements.get(id),createElement:make,hidden:false}, window:{__TAURI__:{core:{invoke:async (name,args)=>{ calls.push(name); requests.push({name,args}); if(name==='manager_set_visibility')return show(args); if(name==='manager_notes')return readNotes(); if(name==='manager_image')return 'data:image/png;base64,test'; return data; }}},addEventListener(){}},setInterval(){}});
  vm.runInContext(source,context);
  return { elements, calls, requests, setShow:fn=>{show=fn;}, setReadNotes:fn=>{readNotes=fn;}, run:code=>vm.runInContext(code,context), setData:value=>{data=value;} };
}
test('read-only selection, settings navigation and empty result', async () => {
  const app = setup(); await new Promise(setImmediate);
  assert.match(app.elements.get('detail').textContent,/선택/);
  app.elements.get('list').children[0].onclick();
  assert.equal(app.elements.get('detail').children.length,2);
  assert.equal(app.elements.get('detail').children[1].innerHTML,'<h1>숨긴 메모</h1><p>내용</p>');
  app.elements.get('settings-nav').onclick();
  assert.equal(app.elements.get('notes-page').hidden,true);
  app.elements.get('notes-nav').onclick(); await new Promise(setImmediate);
  assert.equal(app.elements.get('detail').children.length,2);
  assert.equal(app.elements.get('detail').children[1].innerHTML,'<h1>숨긴 메모</h1><p>내용</p>');
  assert.deepEqual([...new Set(app.calls)],['manager_notes']);
  app.setData([]); await app.run('refresh(true)');
  assert.match(app.elements.get('detail').textContent,/없습니다/);
});

const tick = () => new Promise(setImmediate);
const tool = app => app.elements.get('detail').children[0];
const openButton = app => tool(app).children[0];
test('one common show action precedes unchanged content for every memo type', async () => {
  const app = setup(); await tick();
  const rows = [
    {id:'text',title:'제목',html:'<h1>제목</h1><p>본문</p>'},
    {id:'plain',title:'제목 없는 본문',html:'<p>제목 없는 본문</p>'},
    {id:'empty',title:'빈 메모',html:''},
    {id:'image',title:'이미지',capture:{}},
    {id:'terminal',title:'터미널',kind:'terminal'},
    {id:'agent',title:'에이전트',kind:'agent'}
  ];
  app.setData(rows); await app.run('refresh(true)');
  assert.equal(app.elements.get('detail').children.length,0);
  for (const [index,row] of rows.entries()) {
    app.elements.get('list').children[index].onclick(); await tick();
    assert.equal(app.requests.filter(r=>r.name==='manager_set_visibility').length,index);
    assert.equal(tool(app)['aria-label'],'메모 공통 도구');
    assert.equal(openButton(app).textContent,'화면에 띄우기');
    const detail = app.elements.get('detail').children;
    if (!row.kind && !row.capture) {
      assert.equal(detail.length,2); assert.equal(detail[1].innerHTML,row.html);
    } else assert.equal(detail[1].tagName,'h1');
    await openButton(app).onclick();
    assert.equal(app.requests.at(-1).name,'manager_set_visibility');
    assert.equal(app.requests.at(-1).args.id,row.id);
    assert.equal(openButton(app).disabled,false);
  }
  app.elements.get('settings-nav').onclick();
  assert.equal(app.elements.get('notes-page').hidden,true);
  assert.equal(app.elements.get('settings-page').children.length,0);
});
test('show failure stays visible across polling and supports retry without exposing raw errors', async () => {
  const app = setup(); await tick(); app.elements.get('list').children[0].onclick();
  app.setShow(async()=>{throw Error('private path');});
  await openButton(app).onclick();
  const error = tool(app).children[1];
  assert.equal(error.hidden,false); assert.equal(error.role,'alert');
  assert.match(error.textContent,/변경하지 못했습니다/); assert.doesNotMatch(error.textContent,/private/);
  await app.run('refresh()'); assert.equal(error.hidden,false);
  app.setShow(async({visible})=>visible); await openButton(app).onclick(); assert.equal(error.hidden,true);
});
test('pending actions use the clicked id, prevent duplicate submission and cannot affect a new selection', async () => {
  const app = setup(); await tick();
  app.setData([{id:'a',html:'a'},{id:'b',html:'b'}]); await app.run('refresh(true)');
  let reject;
  app.setShow(()=>new Promise((_,r)=>{reject=r;}));
  app.elements.get('list').children[0].onclick();
  const button = openButton(app), pending = button.onclick();
  assert.equal(button.disabled,true); await button.onclick();
  assert.equal(app.requests.filter(r=>r.name==='manager_set_visibility').length,1);
  assert.equal(app.requests.at(-1).args.id,'a');
  app.elements.get('list').children[1].onclick();
  reject(Error('late')); await pending;
  assert.equal(openButton(app).disabled,false);
  assert.equal(tool(app).children[1].hidden,true);
  assert.equal(app.elements.get('detail').children[1].innerHTML,'b');
});

test('visible status toggles to hide and back for every memo kind', async () => {
  for (const row of [{id:'text'},{id:'image',capture:{}},{id:'agent',kind:'agent'},{id:'terminal',kind:'terminal'}]) {
    const app = setup(); await tick(); app.setData([{...row,visible:true}]); await app.run('refresh(true)');
    app.elements.get('list').children[0].onclick(); await tick();
    assert.equal(openButton(app).textContent,'화면에서 숨기기');
    assert.equal(tool(app).children.filter(child=>!child.hidden).length,1);
    await openButton(app).onclick();
    assert.equal(app.requests.at(-1).args.visible,false);
    assert.equal(openButton(app).textContent,'화면에 띄우기');
    assert.equal(tool(app).children.filter(child=>!child.hidden).length,1);
    await openButton(app).onclick();
    assert.equal(app.requests.at(-1).args.visible,true);
    assert.equal(openButton(app).textContent,'화면에서 숨기기');
  }
});
test('polling external visibility changes keeps the existing controls and inline errors', async () => {
  const app = setup(); await tick(); app.elements.get('list').children[0].onclick();
  const button = openButton(app), tools = tool(app);
  app.setData([{id:'hidden',title:'숨긴 메모',html:'<h1>숨긴 메모</h1><p>내용</p>',capture:null,visible:true}]);
  await app.run('refresh()');
  assert.equal(openButton(app),button); assert.equal(tool(app),tools);
  assert.equal(button.textContent,'화면에서 숨기기');
  app.setShow(async()=>{throw Error('hide failed');}); await button.onclick();
  assert.equal(button.textContent,'화면에서 숨기기'); assert.equal(tools.children[1].hidden,false);
  await app.run('refresh()'); assert.equal(tools.children[1].hidden,false);
});
test('a query started before a visibility change cannot roll back the confirmed result', async () => {
  const app = setup(); await tick(); app.elements.get('list').children[0].onclick();
  let resolve; app.setReadNotes(()=>new Promise(r=>{resolve=r;}));
  const polling = app.run('refresh()');
  await openButton(app).onclick(); assert.equal(openButton(app).textContent,'화면에서 숨기기');
  resolve([{id:'hidden',title:'숨긴 메모',html:'<h1>숨긴 메모</h1><p>내용</p>',capture:null,visible:false}]);
  await polling;
  assert.equal(openButton(app).textContent,'화면에서 숨기기');
});

test('settings sections preserve privacy drafts and selected memo across navigation', async () => {
  const app = setup(); await new Promise(setImmediate);
  const e = id => app.elements.get(id);
  e('list').children[0].onclick();
  e('settings-nav').onclick();
  e('settings-privacy').draft = 'unsaved sharing choice';
  e('settings-skills-nav').onclick();
  assert.equal(e('settings-privacy').hidden, true);
  assert.equal(e('settings-skills').hidden, false);
  assert.equal(e('settings-skills-nav')['aria-current'], 'page');
  assert.equal(e('settings-privacy-nav')['aria-current'], 'false');
  e('notes-nav').onclick(); e('settings-nav').onclick();
  assert.equal(e('settings-skills').hidden, false);
  e('settings-privacy-nav').onclick();
  assert.equal(e('settings-privacy').hidden, false);
  assert.equal(e('settings-skills').hidden, true);
  assert.equal(e('settings-privacy').draft, 'unsaved sharing choice');
  assert.equal(e('settings-privacy-nav')['aria-current'], 'page');
  e('notes-nav').onclick();
  assert.match(e('detail').children[1].innerHTML, /숨긴 메모/);
  assert.deepEqual([...new Set(app.calls)], ['manager_notes']);
});
