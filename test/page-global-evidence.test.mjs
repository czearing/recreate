import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildPageGlobalContent,
  buildPageGlobalLayout,
} from '../src/page-global-evidence.mjs';

test('emits readable leaf text outside component roots', () => {
  const nodes = [
    {
      path: 'doc(0)>main',
      tag: 'main',
      text: 'Hero',
      rect: { width: 100, height: 50 },
    },
    {
      path: 'doc(0)>main>h1',
      parentPath: 'doc(0)>main',
      tag: 'h1',
      text: 'Hero',
      rect: { width: 100, height: 50 },
      style: { fontSize: '40px' },
    },
    {
      path: 'doc(0)>section>button',
      tag: 'button',
      text: 'Inside component',
      rect: { width: 100, height: 32 },
    },
  ];
  const content = buildPageGlobalContent(nodes, ['doc(0)>section']);
  assert.deepEqual(content.map(({ text }) => text), ['Hero']);
  assert.equal(content[0].style.fontSize, '40px');
  const layout = buildPageGlobalLayout(nodes, ['doc(0)>section'], content);
  assert.deepEqual(layout.map(({ path }) => path), ['doc(0)>main']);
});
