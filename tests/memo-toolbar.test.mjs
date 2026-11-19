import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

const script = readFileSync(new URL('../ui/memo-toolbar.js', import.meta.url), 'utf8');
function mount(kind) {
  const nodes = [];
  const createElement = () => {
    const node = {dataset: {}, attributes: {}, children: [],
      append(...children) { this.children.push(...children); },
      setAttribute(key, value) { this.attributes[key] = value; },
    };
    nodes.push(node);
    return node;
  };
  const controls = createElement();
  controls.dataset.kind = kind;
  const window = {};
  vm.runInNewContext(script, {document: {createElement, currentScript: {parentElement: controls}}, window});
  return {nodes, api: window.jajanMemoToolbar};
}

test('all memo types share accessible controls and retain their supported actions', () => {
  for (const kind of ['text', 'image', 'terminal', 'web']) {
    const {nodes, api} = mount(kind === 'image' ? 'text' : kind);
    api.setKind(kind);
    const byId = id => nodes.find(n => n.id === id);
    assert.equal(Boolean(byId('copy') && !byId('copy').hidden), kind !== 'terminal');
    assert.equal(Boolean(byId('font-size') && !byId('font-size').hidden), kind !== 'image');
    assert.equal(byId('warp'), undefined);
    assert.equal(byId('toolbar-next').attributes['aria-controls'], 'toolbar-items');
    for (const node of nodes.filter(n => n.type === 'button')) {
      assert.ok(node.attributes['aria-label']);
      assert.match(node.innerHTML, /aria-hidden="true"/);
    }
  }
});
