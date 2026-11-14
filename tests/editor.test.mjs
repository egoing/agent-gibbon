import test from 'node:test';
import assert from 'node:assert/strict';
import { setup } from './memo-app-fixture.mjs';
import { feedbackView } from '../ui/feedback-state.mjs';

test('text surface is always editable and clean reads never serialize/save/export', async()=>{
  const app=await setup({markdown:'* 원문\r\n\r\n'});
  assert.equal(app.element('editor').hidden,false);
  assert.equal(app.element('save').disabled,true);
  assert.equal(app.busy(),false);
  for(const key of ['s','S'])await app.key(key,{ctrlKey:true});
  await app.refresh(); await app.leave();
  assert.equal(app.calls.some(([n])=>['save_note','export_note'].includes(n)),false);
  assert.equal(app.note().markdown,'* 원문\r\n\r\n');
});
test('dirty draft survives external revision and save conflict; explicit cancel reveals latest', async()=>{
  const app=await setup();app.input('초안');
  app.update({revision:2,markdown:'외부'});await app.refresh();
  assert.equal(app.draft(),'초안');assert.equal(app.busy(),true);
  await app.action('save');
  assert.equal(app.calls.find(([n])=>n==='save_note')[1].expectedRevision,1);
  assert.equal(app.draft(),'초안');assert.match(app.view().message,/외부/);
  await app.action('cancel');assert.equal(app.draft(),'외부');assert.equal(app.busy(),false);
  assert.equal(app.element('editor').hidden,false);
});
test('explicit save preserves editor and clears dirty; failure preserves retryable draft', async()=>{
  const app=await setup();app.input('저장할 초안');app.failSave('연결 파일 충돌');
  await app.action('save');assert.equal(app.draft(),'저장할 초안');assert.equal(app.busy(),true);assert.equal(app.locked(),false);
  app.failSave(null);await app.element('save').listeners.click();await app.settle();
  assert.equal(app.note().markdown,'저장할 초안');assert.equal(app.view().error,false);assert.equal(app.busy(),false);assert.equal(app.element('editor').hidden,false);
});
test('save shortcuts share explicit save on both platforms and never export text', async()=>{
  for(const platform of ['MacIntel','Win32']){
    const app=await setup({}, {}, platform);app.input('단축키');
    await app.key('s',{ctrlKey:platform==='Win32',metaKey:platform==='MacIntel'});
    assert.equal(app.note().markdown,'단축키');assert.equal(app.calls.some(([n])=>n==='export_note'),false);
  }
});
test('composition protects save/cancel/remote refresh and updater busy until composition ends',async()=>{
  const app=await setup();app.input('한글');app.composition(true);
  app.update({revision:2,markdown:'수신'});await app.refresh();
  await app.key('s',{metaKey:true});await app.key('Escape');await app.action('cancel');
  assert.equal(app.draft(),'한글');assert.equal(app.calls.some(([n])=>n==='save_note'),false);assert.equal(app.busy(),true);
  app.composition(false);await app.action('save');assert.match(app.view().message,/외부/);
});
test('IME keycode and repeat/modified shortcuts never save',async()=>{
  const app=await setup();app.input('한글');
  for(const extra of [{isComposing:true},{keyCode:229},{repeat:true},{altKey:true},{shiftKey:true}])await app.key('s',{ctrlKey:true,...extra});
  assert.equal(app.calls.some(([n])=>n==='save_note'),false);
});
test('pending autosave coalesces requests without locking input and saves newer draft next',async()=>{
  const app=await setup();app.input('첫 저장');let finish;app.delaySave(new Promise(r=>finish=r));
  const pending=app.flushAutosave();await app.settle();assert.equal(app.locked(),false);
  app.input('저장 중 새 입력');const duplicate=app.action('save');await app.action('export');await app.action('cancel');
  assert.equal(app.calls.filter(([n])=>n==='save_note').length,1);assert.equal(app.draft(),'저장 중 새 입력');
  finish();await pending;await duplicate;assert.equal(app.note().markdown,'첫 저장');assert.equal(app.busy(),true);
  await app.flushAutosave();assert.equal(app.note().markdown,'저장 중 새 입력');assert.equal(app.busy(),false);
  assert.equal(app.calls.filter(([n])=>n==='save_note')[1][1].expectedRevision,2);
});
test('unsupported source protection never becomes a dirty save',async()=>{
  const app=await setup();app.protect(true);
  await app.key('s',{ctrlKey:true});assert.equal(app.calls.some(([n])=>n==='save_note'),false);
  assert.equal(app.element('save').disabled,true);assert.equal(app.busy(),false);
});
test('fold/hide/restore and theme-only refresh retain local draft and source base',async()=>{
  const app=await setup();app.input('남겨둘 초안');
  app.state.folded=true;await app.pollFold();app.state.visible=false;await app.refresh();
  app.update({color:'#ffffff'});app.state.visible=true;app.state.folded=false;await app.pollFold();await app.refresh();
  assert.equal(app.draft(),'남겨둘 초안');assert.equal(typeof app.styles['--background'],'string');
  assert.equal(app.calls.filter(([n])=>n==='enter_note_editor').length,1);
});
test('first dirty input requests entry height once, clean open and refresh do not',async()=>{
  const app=await setup();assert.equal(app.calls.some(([n])=>n==='enter_note_editor'),false);
  app.input('a');app.input('ab');await app.refresh();await app.action('cancel');app.input('c');
  assert.equal(app.calls.filter(([n])=>n==='enter_note_editor').length,1);
});
test('dirty state defers acknowledgement; clean focus departure confirms only observed revision',async()=>{
  const app=await setup();app.update({revision:2,change_html:'<mark>변경</mark>'});app.state.highlight_revision=2;await app.refresh();
  app.input('초안');await app.leave();assert.equal(app.calls.some(([n])=>n==='acknowledge_feedback'),false);
  await app.action('cancel');await app.leave();assert.equal(app.calls.filter(([n])=>n==='acknowledge_feedback').at(-1)[1].revision,2);
});
test('selected copy preserves native shortcut, copy button copies draft without save',async()=>{
  const app=await setup();app.input('작성중');app.state.selection='선택';
  let prevented=false;app.events.keydown({key:'c',metaKey:true,preventDefault(){prevented=true;}});assert.equal(prevented,false);
  await app.element('copy').listeners.click();assert.equal(app.calls.find(([n])=>n==='clipboard-text')[1],'선택');
  assert.equal(app.calls.some(([n])=>n==='save_note'),false);
});
test('image notes keep native copy and shortcut export with no text editor',async()=>{
  const app=await setup({capture:{width:100,height:100}});assert.equal(app.element('editor').hidden,true);
  await app.key('s',{ctrlKey:true});assert.equal(app.calls.some(([n])=>n==='export_note'),true);
  await app.element('copy').listeners.click();assert.equal(app.calls.some(([n])=>n==='copy_note_image'),true);
});
test('font sizing, monitor direction and native rollover remain independent of editing',async()=>{
  const app=await setup();app.input('초안');
  for(const size of [14,16,18,12]){app.element('font-size').listeners.click();assert.equal(app.styles['--font-size'],`${size}px`);}
  app.state.warp_right=true;await app.refresh();assert.equal(app.element('warp').attributes['data-direction'],'right');
  app.nativeEvents['note-pointer-over']({payload:true});assert.equal(app.classes.has('pointer-over'),true);assert.equal(app.draft(),'초안');
});
test('Escape flushes changes before hiding and close refuses to discard a failed save',async()=>{
  const app=await setup();app.input('초안');await app.key('Escape');assert.equal(app.note().markdown,'초안');assert.equal(app.calls.some(([n])=>n==='hide_note'),true);
  app.input('실패 초안');app.failSave('원본 충돌');await app.element('close').listeners.click();assert.equal(app.calls.some(([n])=>n==='close_note'),false);assert.equal(app.draft(),'실패 초안');
  app.failSave(null);await app.element('close').listeners.click();assert.equal(app.calls.some(([n])=>n==='close_note'),true);
});
test('autosave Feedback shows status and exposes recovery only on failure',()=>{
  const view=feedbackView({unified:true,editing:true,saving:false,exporting:false,composition:true});
  assert.equal(view.message,'입력 중…');assert.deepEqual(view.buttons,[]);
  const failed=feedbackView({unified:true,editing:true,autosaveBlocked:true});assert.deepEqual(failed.buttons.map(b=>b.label),['변경 취소','다시 저장']);
});
test('autosave coalesces latest input and never writes an unchanged note',async()=>{
  const app=await setup();await app.flushAutosave();assert.equal(app.calls.some(([n])=>n==='save_note'),false);
  app.input('a');app.input('ab');app.input('abc');assert.equal(app.calls.some(([n])=>n==='save_note'),false);
  await app.flushAutosave();assert.equal(app.note().markdown,'abc');assert.equal(app.calls.filter(([n])=>n==='save_note').length,1);
});
test('composition defers autosave until final input is committed',async()=>{
  const app=await setup();app.input('ㅎ');app.composition(true);await app.flushAutosave();assert.equal(app.calls.some(([n])=>n==='save_note'),false);
  app.input('한글');app.composition(false);await app.flushAutosave();assert.equal(app.note().markdown,'한글');
});
test('autosave failure pauses retries and retains newer input until explicit retry',async()=>{
  const app=await setup();app.failSave('파일 외부 변경');app.input('초안');await app.flushAutosave();
  app.input('보존할 새 초안');await app.flushAutosave();assert.equal(app.calls.filter(([n])=>n==='save_note').length,1);assert.equal(app.draft(),'보존할 새 초안');
  app.failSave(null);await app.action('save');assert.equal(app.note().markdown,'보존할 새 초안');assert.equal(app.busy(),false);
});
test('undo while a write is pending is saved as the following revision',async()=>{
  const app=await setup();app.input('새 입력');let finish;app.delaySave(new Promise(r=>finish=r));
  const pending=app.flushAutosave();await app.settle();app.input('# 원문');finish();await pending;
  assert.equal(app.note().markdown,'새 입력');assert.equal(app.draft(),'# 원문');assert.equal(app.busy(),true);
  await app.flushAutosave();assert.equal(app.note().markdown,'# 원문');assert.equal(app.busy(),false);
});

test('a revision first seen on focus departure remains unread',async()=>{
  const app=await setup({}, {focused:true});await app.refresh();
  app.state.focused=false;app.update({revision:2,markdown:'새 수신',html:'<p>새 수신</p>'});app.state.highlight_revision=2;await app.refresh();
  assert.equal(app.calls.some(([n])=>n==='acknowledge_feedback'),false);
  await app.leave();assert.equal(app.calls.filter(([n])=>n==='acknowledge_feedback').at(-1)[1].revision,2);
});
test('stale snapshots after successful save cannot replace the saved editor base',async()=>{
  const app=await setup();app.input('최신 저장');await app.action('save');
  app.update({revision:1,markdown:'늦은 응답'});await app.refresh();assert.equal(app.draft(),'최신 저장');
});
test('hidden notes suppress inline feedback delivery until explicitly visible',async()=>{
  const app=await setup({}, {visible:false});assert.equal(app.calls.some(([n])=>n==='present_feedback'),false);
  app.state.visible=true;await app.refresh();assert.equal(app.calls.some(([n])=>n==='present_feedback'),true);
});
test('native pointer messages target the owning memo and do not mark content edited',async()=>{
  const app=await setup();assert.equal(app.nativeTargets['note-pointer-over'].label,'note-fixture');
  const before=app.calls.filter(([n])=>n==='content_action').length;
  app.nativeEvents['note-pointer-over']({payload:true});app.nativeEvents['note-pointer-over']({payload:false});
  assert.equal(app.calls.filter(([n])=>n==='content_action').length,before);assert.equal(app.busy(),false);
});
test('copy ignores removed diff text and does not acknowledge a change',async()=>{
  const app=await setup({html:'<p>실제 본문</p>',change_html:'<mark>삭제 이력</mark>'});
  await app.element('copy').listeners.click();assert.equal(app.calls.find(([n])=>n==='clipboard-text')[1],'실제 본문');
  assert.equal(app.calls.some(([n])=>n==='acknowledge_feedback'),false);
});
test('clipboard failure and warp failure report actual errors without erasing draft',async()=>{
  const app=await setup();app.input('초안 보존');app.failCopy('복사 실패');await app.element('copy').listeners.click();assert.equal(app.view().message,'복사 실패');
  app.failWarp('이동 실패');await app.element('warp').listeners.click();assert.equal(app.view().message,'이동 실패');assert.equal(app.draft(),'초안 보존');
});
test('image export failures preserve state and can retry',async()=>{
  const app=await setup({capture:{width:10,height:10}});app.failExport('내보내기 실패');await app.key('s',{ctrlKey:true});assert.equal(app.view().message,'내보내기 실패');assert.equal(app.busy(),false);
  app.failExport(null);await app.key('s',{ctrlKey:true});assert.equal(app.calls.filter(([n])=>n==='export_note').length,2);
});
test('failed automatic acknowledgement keeps decoration and can retry',async()=>{
  const app=await setup({change_html:'<mark>수신</mark>'},{highlight_revision:1});app.failAcknowledgement('확인 실패');await app.leave();assert.equal(app.view().message,'확인 실패');
  app.failAcknowledgement(null);await app.leave();assert.equal(app.calls.filter(([n])=>n==='acknowledge_feedback').length,2);
});
test('explicit region input waits for visible/unfolded state and focuses once',async()=>{
  const app=await setup({edit_on_open:true},{visible:false,folded:true});assert.equal(app.calls.some(([n])=>n==='editor-focus'),false);
  app.state.visible=true;await app.refresh();assert.equal(app.calls.some(([n])=>n==='editor-focus'),false);
  app.state.folded=false;await app.pollFold();await app.refresh();await app.refresh();assert.equal(app.calls.filter(([n])=>n==='editor-focus').length,1);
});
test('normal CLI-created and capture notes never request editor focus',async()=>{
  for(const initial of [{},{capture:{width:10,height:10},edit_on_open:true}]){
    const app=await setup(initial);await app.refresh();assert.equal(app.calls.some(([n])=>n==='editor-focus'),false);
  }
});
