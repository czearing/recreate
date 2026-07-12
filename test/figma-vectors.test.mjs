import assert from 'node:assert/strict';
import test from 'node:test';
import {
  geometryBlobToSvg,
  resolveFigmaGeometry,
} from '../src/figma-vectors.mjs';

const commandBlob = () => {
  const buffer = new ArrayBuffer(1 + 8 + 1 + 8 + 1);
  const bytes = new Uint8Array(buffer);
  const view = new DataView(buffer);
  let offset = 0;
  bytes[offset++] = 1;
  view.setFloat32(offset, 1, true);
  view.setFloat32(offset + 4, 2, true);
  offset += 8;
  bytes[offset++] = 2;
  view.setFloat32(offset, 3, true);
  view.setFloat32(offset + 4, 4, true);
  offset += 8;
  bytes[offset] = 0;
  return bytes;
};

test('decodes Figma geometry blobs into SVG paths', () => {
  assert.deepEqual(geometryBlobToSvg(commandBlob()), {
    d: 'M1 2 L3 4 Z',
    error: null,
  });
});

test('keeps blob identity and reports missing geometry explicitly', () => {
  assert.deepEqual(resolveFigmaGeometry([
    { windingRule: 'NONZERO', commandsBlob: 0, styleID: 3 },
    { windingRule: 'EVENODD', commandsBlob: 5, styleID: 4 },
  ], [commandBlob()]), [
    {
      windingRule: 'NONZERO',
      styleId: 3,
      commandsBlob: 0,
      d: 'M1 2 L3 4 Z',
      error: null,
    },
    {
      windingRule: 'EVENODD',
      styleId: 4,
      commandsBlob: 5,
      d: null,
      error: 'Missing geometry blob.',
    },
  ]);
});
