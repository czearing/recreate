import assert from 'node:assert/strict';
import test from 'node:test';
import { anchorParent } from '../anchor.mjs';

test('anchors to the clicked repeated control before the shared fallback', () => {
  const activeParent = { id: 'card-b' };
  const fallbackParent = { id: 'card-a' };
  const root = {
    body: { id: 'body' },
    querySelector(selector) {
      return selector.includes('active')
        ? { parentElement: activeParent }
        : { parentElement: fallbackParent };
    },
  };
  assert.equal(anchorParent(root, 4), activeParent);
});

test('uses the state trigger and then body when active identity is stale', () => {
  const fallbackParent = { id: 'card-a' };
  const root = {
    body: { id: 'body' },
    querySelector(selector) {
      return selector.includes('active') ? null : { parentElement: fallbackParent };
    },
  };
  assert.equal(anchorParent(root, 4), fallbackParent);
  root.querySelector = () => null;
  assert.equal(anchorParent(root, 4), root.body);
});
