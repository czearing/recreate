import assert from 'node:assert/strict';
import test from 'node:test';

import { compareNativeState } from '../src/compare-native-state.mjs';

const node = (label, x, opacity = '1') => ({
  tag: 'button',
  text: label,
  rect: { x, y: 0, width: 10, height: 10 },
  style: { display: 'block', opacity },
});

test('matches duplicate identities by nearest geometry', () => {
  const result = compareNativeState(
    { nodes: [node('More', 10), node('More', 100)] },
    { nodes: [node('More', 98), node('More', 12)] },
  );
  assert.equal(result.matched, 2);
  assert.equal(result.maxDeltaPx, 2);
});

test('reports hidden controls separately from painted geometry', () => {
  const result = compareNativeState(
    { nodes: [node('Create', 10, '0'), node('Open', 20)] },
    { nodes: [node('Create', 80, '0'), node('Open', 21)] },
  );
  assert.equal(result.maxDeltaPx, 70);
  assert.equal(result.painted.maxDeltaPx, 1);
});
