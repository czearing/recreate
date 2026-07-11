import assert from 'node:assert/strict';
import test from 'node:test';

import {
  compactListeners,
  implementationNodes,
} from '../src/evidence-filter.mjs';

test('removes advertising subtrees from implementation evidence', () => {
  const nodes = [
    { path: 'doc(0)>html', tag: 'html', attrs: {} },
    {
      path: 'doc(0)>html>main',
      parentPath: 'doc(0)>html',
      tag: 'main',
      attrs: {},
      visible: true,
    },
    {
      path: 'doc(0)>html>div',
      tag: 'div',
      attrs: { class: 'adunitwrapper' },
    },
    {
      path: 'doc(0)>html>div>iframe',
      tag: 'iframe',
      attrs: {},
      text: 'Advertisement',
    },
    { path: 'doc(1)>html', tag: 'html', attrs: {} },
  ];

  assert.deepEqual(
    implementationNodes(nodes).map((node) => node.path),
    ['doc(0)>html', 'doc(0)>html>main'],
  );
});

test('deduplicates listeners and removes known ad scripts', () => {
  const listener = {
    target: 'window',
    type: 'keydown',
    sourceUrl: 'https://example.test/game.js',
    lineNumber: 1,
    columnNumber: 2,
    sourceStatus: 'located',
  };
  const adListener = {
    ...listener,
    sourceUrl: 'https://cdn.ampproject.org/ads.js',
  };

  const compact = compactListeners([listener, listener, adListener]);
  assert.equal(compact.length, 1);
  assert.equal(compact[0].sourceUrl, listener.sourceUrl);
});
