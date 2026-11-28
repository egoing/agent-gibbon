import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';

const ui = new URL('../ui/', import.meta.url);
const read = file => readFileSync(new URL(file, ui), 'utf8');
const catalog = JSON.parse(read('messages.json'));
const variables = text => Array.from(text.matchAll(/\{(\w+)\}/g), m => m[1]).sort();

test('all messages provide the same variables in Korean, English and Japanese', () => {
  for (const [key, translations] of Object.entries(catalog)) {
    assert.deepEqual(Object.keys(translations).sort(), ['en', 'ja', 'ko'], key);
    for (const text of Object.values(translations)) {
      assert.ok(text.trim(), key);
      assert.deepEqual(variables(text), variables(key), key);
    }
  }
});

test('every local HTML entry loads the language bootstrap before its application scripts', () => {
  for (const file of readdirSync(ui).filter(file => file.endsWith('.html'))) {
    const scripts = [...read(file).matchAll(/<script[^>]+src="([^"]+)"/g)].map(m => m[1]);
    assert.deepEqual(scripts.slice(0, 2), ['generated/messages.js', 'language.js'], file);
  }
});

test('every explicitly bound source literal has a translation', () => {
  for (const file of readdirSync(ui).filter(file => file.endsWith('.js'))) {
    const source = read(file);
    for (const match of source.matchAll(/(?:jajanLanguage|language)\.(?:bind\([^\n]*?,\s*'(?:textContent|title|aria-label)',\s*|(?:t|msg)\(\s*)'([^'\n]*[가-힣][^'\n]*)'(?=\s*[,)]|\s*\?)/g)) {
      assert.ok(Object.hasOwn(catalog, match[1]), file + ': ' + match[1]);
    }
  }
});
