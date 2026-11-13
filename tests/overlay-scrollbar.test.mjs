import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const source = await readFile(new URL('../ui/overlay-scrollbar.js', import.meta.url), 'utf8');
const { scrollGeometry } = await import(`data:text/javascript,${encodeURIComponent(source)}`);
test('overlay thumb reaches both ends without escaping the content track', () => {
  for (const view of [10, 100, 300]) {
    const total = view * 5;
    const start = scrollGeometry(view, total, -50);
    const end = scrollGeometry(view, total, total);
    assert.equal(start.top, 0);
    assert.equal(end.top + end.size, view - 4);
    assert.equal(end.range, total - view);
    assert.ok(end.size <= view - 4);
  }
});
test('short and empty notes have no scroll range; long notes retain a draggable thumb', () => {
  assert.equal(scrollGeometry(200, 100, 0).range, 0);
  assert.deepEqual(scrollGeometry(0, 0, 0), { range: 0, size: 0, travel: 0, top: 0 });
  assert.equal(scrollGeometry(200, 100000, 0).size, 24);
});
