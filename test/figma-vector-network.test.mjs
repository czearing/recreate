import assert from 'node:assert/strict';
import test from 'node:test';
import { resolveVectorNetwork } from '../src/figma-vector-network.mjs';

function lineNetworkBlob() {
  const buffer = new ArrayBuffer(64);
  const view = new DataView(buffer);
  let offset = 0;
  const uint = (value) => {
    view.setUint32(offset, value, true);
    offset += 4;
  };
  const float = (value) => {
    view.setFloat32(offset, value, true);
    offset += 4;
  };
  uint(2);
  uint(1);
  uint(0);
  uint(0);
  float(0);
  float(0);
  uint(0);
  float(10);
  float(0);
  uint(0);
  uint(0);
  float(0);
  float(0);
  uint(1);
  float(0);
  float(0);
  return new Uint8Array(buffer);
}

test('decodes open Figma vector networks into SVG paths', () => {
  const result = resolveVectorNetwork({
    size: { x: 10, y: 1 },
    vectorData: {
      vectorNetworkBlob: 0,
      normalizedSize: { x: 10, y: 1 },
    },
  }, [lineNetworkBlob()]);
  assert.deepEqual(result, {
    blobIndex: 0,
    normalizedSize: { x: 10, y: 1 },
    vertexCount: 2,
    segmentCount: 1,
    regionCount: 0,
    paths: [{ windingRule: 'NONZERO', d: 'M0 0 L10 0' }],
    error: null,
  });
});

test('reports missing vector blobs explicitly', () => {
  assert.deepEqual(resolveVectorNetwork({
    vectorData: { vectorNetworkBlob: 4 },
  }, []), {
    blobIndex: 4,
    paths: [],
    error: 'Missing vector network blob.',
  });
});
