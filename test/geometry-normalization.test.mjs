import assert from 'node:assert/strict';
import test from 'node:test';

import { normalizeSnapshotGeometry } from '../src/geometry-normalization.mjs';

test('normalizes device-scaled snapshot bounds to runtime CSS pixels', () => {
  const decoded = {
    documents: [{ contentWidth: 2160, contentHeight: 1350 }],
    nodes: [
      { rect: { x: 0, y: 0, width: 2160, height: 1350, right: 2160, bottom: 1350 } },
      { rect: { x: 265.5, y: 833, width: 792, height: 234, right: 1057.5, bottom: 1067 } },
    ],
  };
  const scale = normalizeSnapshotGeometry(decoded, {
    viewport: { width: 1440, height: 900 },
    scroll: { width: 1440, height: 900 },
  });
  assert.equal(scale, 1.5);
  assert.deepEqual(decoded.nodes[1].rect, {
    x: 177,
    y: 555.333333,
    width: 528,
    height: 156,
    right: 705,
    bottom: 711.333333,
  });
});

test('preserves legitimate horizontal overflow', () => {
  const decoded = {
    documents: [{ contentWidth: 2160, contentHeight: 900 }],
    nodes: [{ rect: { x: 0, y: 0, width: 2160, height: 900, right: 2160, bottom: 900 } }],
  };
  assert.equal(normalizeSnapshotGeometry(decoded, {
    viewport: { width: 1440, height: 900 },
    scroll: { width: 2160, height: 900 },
  }), 1);
});
