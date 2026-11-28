import { sourceLanguageWindow } from './fixtures/language.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const read = name => readFileSync(new URL(`../ui/${name}`, import.meta.url), 'utf8');

test('open and repeat reuse the same icon nodes and expose the next action', () => {
  const title = {};
  const classes = new Set(['closing']);
  const classList = {
    add: name => classes.add(name),
    remove: name => classes.delete(name),
    toggle: (name, on) => on ? classes.add(name) : classes.delete(name),
  };
  const window = {};
  vm.runInNewContext(read('direction-guide.js'), sourceLanguageWindow({
    window, document: { querySelector: selector => {
      assert.equal(selector, 'title');
      return title;
    }, body: { classList } },
  }));
  for (const restoring of [false, false, true, true, false]) {
    window.showDirectionGuide(restoring);
    assert.match(title.textContent, restoring ? /아래로 전체 복원/ : /아래로 전체 숨기기/);
    assert.match(title.textContent, /Esc 키로 취소/);
    assert.match(title.textContent, /위로 메모 모아보기/);
    assert.match(title.textContent, /왼쪽으로 화면 빈자리 배치/);
    assert.equal(classes.has('restoring'), restoring);
    assert.equal(classes.has('open'), true);
    assert.equal(classes.has('closing'), false);
  }
  const references = [...read('direction-guide.html').matchAll(/<use [^>]*href="([^"#]+)#([^"]+)"/g)];
  assert.deepEqual(references.map(match => match[2]), ['overview', 'right', 'left', 'hide', 'restore']);
  for (const [, file, id] of references) assert.ok(read(file).includes(`<g id="${id}"`));
});
