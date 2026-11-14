import test from 'node:test';
import assert from 'node:assert/strict';
import { setup } from './memo-app-fixture.mjs';
test('toolbar blank drag delegates to entrance-cancelling native command',async()=>{
  const app=await setup();let prevented=false;
  app.events.mousedown({button:0,target:{closest:selector=>selector==='#controls'?{}:null},preventDefault(){prevented=true;}});
  assert.equal(prevented,true);assert.equal(app.calls.filter(([n])=>n==='drag_note').length,1);
});
test('body selection, buttons and nonprimary gestures never drag the note',async()=>{
  const app=await setup();
  for(const [button,toolbar,control] of [[0,false,false],[0,true,true],[1,true,false],[2,true,false]]){
    app.events.mousedown({button,target:{closest:selector=>selector==='#controls'?(toolbar?{}:null):(control?{}:null)},preventDefault(){assert.fail('must preserve native interaction');}});
  }
  assert.equal(app.calls.some(([n])=>n==='drag_note'),false);
});
