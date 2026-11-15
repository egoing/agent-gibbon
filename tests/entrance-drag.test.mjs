import test from 'node:test';
import assert from 'node:assert/strict';
import { setup } from './memo-app-fixture.mjs';
test('memo content does not register an immediate mousedown drag handler',async()=>{
  const app=await setup();
  // Pointer movement and toolbar double clicks belong to the shared toolbar.
  // A document mousedown handler would consume both selection and double click.
  assert.equal(app.events.mousedown,undefined);
  assert.equal(app.calls.some(([name])=>name==='drag_note'),false);
});
