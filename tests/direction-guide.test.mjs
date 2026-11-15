import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import vm from 'node:vm';

test('direction guide switches the eye, reduced-motion asset and accessible action on every opening', () => {
  const image = {}, source = {}, classes = new Set(['closing']);
  const window = {};
  vm.runInNewContext(readFileSync(new URL('../ui/direction-guide.js', import.meta.url), 'utf8'), {
    window, performance: { now: () => 42 },
    document: { querySelector: selector => selector === 'img' ? image : source,
      body: { classList: { remove: name => classes.delete(name) } } },
  });
  for (const restoring of [false, true, false, true]) {
    classes.add('closing');
    window.showDirectionGuide(restoring);
    const name = restoring ? 'direction-guide-restore' : 'direction-guide';
    assert.equal(image.src, `${name}.svg?open=42`);
    assert.equal(source.srcset, `${name}-still.png`);
    assert.match(image.alt, restoring ? /아래로 전체 복원/ : /아래로 전체 숨기기/);
    assert.match(image.alt, /Esc 키로 취소/);
    assert.equal(classes.has('closing'), false);
    assert.ok(existsSync(new URL(`../ui/${source.srcset}`, import.meta.url)));
  }
  assert.notEqual(readFileSync(new URL('../ui/direction-guide.svg', import.meta.url), 'utf8'),
    readFileSync(new URL('../ui/direction-guide-restore.svg', import.meta.url), 'utf8'));
});
