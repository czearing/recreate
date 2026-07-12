import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { createRequire } from 'node:module';
import { decodeFigmaKiwi } from '../src/figma-kiwi.mjs';
import { parseFigmaSource } from '../src/figma-url.mjs';
import { writeFigmaEvidence } from '../src/figma-evidence.mjs';
import {
  collectFigmaImageHashes,
  localizeFigmaImages,
} from '../src/figma-assets.mjs';

const require = createRequire(import.meta.url);
const kiwi = require('kiwi-schema');
const pako = require('pako');

test('classifies Figma URLs without treating normal sites as Figma', () => {
  const community = parseFigmaSource(
    'https://www.figma.com/community/file/12345/Test-File',
  );
  assert.equal(community.kind, 'figma-community');
  assert.equal(community.fileId, '12345');
  assert.match(community.captureUrl, /embed\.figma\.com\/file\/12345/);
  assert.equal(
    parseFigmaSource('https://www.figma.com/design/abc123/Test?node-id=1-2').kind,
    'figma-cloud',
  );
  assert.equal(parseFigmaSource('https://example.com/spec'), null);
});

test('extracts and deduplicates Figma image hashes', () => {
  const hash = Object.fromEntries(
    Array.from({ length: 20 }, (_, index) => [index, index]),
  );
  assert.deepEqual(collectFigmaImageHashes([
    { fillPaints: [{ image: { hash } }] },
    {
      symbolData: {
        nested: {
          paint: {
            image: { hash: Uint8Array.from({ length: 20 }, (_, index) => index) },
          },
        },
      },
    },
  ]), ['000102030405060708090a0b0c0d0e0f10111213']);
});

test('persists explicit asset errors without discarding graph evidence', async () => {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'site-spec-assets-'));
  fs.mkdirSync(path.join(outDir, 'evidence', 'figma'), { recursive: true });
  const result = await localizeFigmaImages({
    cdp: {
      send: async (method) => {
        assert.equal(method, 'Runtime.evaluate');
        return { result: { value: {} } };
      },
    },
    frameId: 'frame',
    outDir,
    source: { imageBatchUrl: 'https://example.com/images' },
    nodes: [{
      fillPaints: [{
        image: { hash: Uint8Array.from({ length: 20 }, (_, index) => index) },
      }],
    }],
  });
  assert.equal(result.complete, false);
  assert.equal(result.errors.length, 1);
  assert.ok(fs.existsSync(path.join(outDir, result.manifest)));
});

test('decodes a fig-kiwi container using its embedded schema', () => {
  const schema = kiwi.parseSchema(
    'struct Node { string type; string name; } ' +
    'message Message { Node[] nodeChanges = 1; }',
  );
  const compiled = kiwi.compileSchema(schema);
  const schemaChunk = pako.deflateRaw(kiwi.encodeBinarySchema(schema));
  const dataChunk = pako.deflateRaw(compiled.encodeMessage({
    nodeChanges: [{ type: 'FRAME', name: 'Card' }],
  }));
  const chunks = [schemaChunk, dataChunk];
  const size = 12 + chunks.reduce((count, chunk) => count + 4 + chunk.length, 0);
  const bytes = new Uint8Array(size);
  bytes.set(new TextEncoder().encode('fig-kiwi'));
  const view = new DataView(bytes.buffer);
  view.setUint32(8, 106, true);
  let offset = 12;
  for (const chunk of chunks) {
    view.setUint32(offset, chunk.length, true);
    offset += 4;
    bytes.set(chunk, offset);
    offset += chunk.length;
  }
  const decoded = decodeFigmaKiwi(bytes);
  assert.equal(decoded.version, 106);
  assert.deepEqual(decoded.message.nodeChanges, [{ type: 'FRAME', name: 'Card' }]);
});

test('writes page sections and omits hidden backing pages by default', () => {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'site-spec-figma-'));
  const node = (sessionID, localID, type, name, parent, extra = {}) => ({
    guid: { sessionID, localID },
    parentIndex: parent
      ? { guid: { sessionID: parent[0], localID: parent[1] } }
      : undefined,
    type,
    name,
    visible: extra.visible ?? true,
    ...extra,
  });
  const decoded = {
    version: 106,
    schemaDefinitionCount: 10,
    message: {
      nodeChanges: [
        node(0, 0, 'DOCUMENT', 'Document'),
        node(1, 1, 'CANVAS', 'Internal', [0, 0], { visible: false }),
        node(1, 2, 'SYMBOL', 'Private', [1, 1]),
        node(2, 1, 'CANVAS', 'Page', [0, 0]),
        node(2, 2, 'FRAME', 'Card', [2, 1], {
          fillPaints: [{ type: 'SOLID', color: { r: 1, g: 0, b: 0, a: 1 } }],
        }),
        node(2, 3, 'TEXT', 'Title', [2, 2], {
          textData: { characters: 'Hello' },
        }),
        node(3, 1, 'VARIABLE_SET', 'Theme', [0, 0]),
        node(3, 2, 'VARIABLE', 'Color', [0, 0], {
          variableSetID: { sessionID: 3, localID: 1 },
          variableDataValues: {
            light: '#fff',
            dark: '#000',
            description: 'A sufficiently long exact variable payload for reference sharding.',
          },
        }),
      ],
    },
  };
  const index = writeFigmaEvidence({
    outDir,
    source: { kind: 'figma-community', fileId: '123' },
    decoded,
    byteLength: 100,
    profile: 'implementation',
  });
  assert.equal(index.pages[0].omittedFromImplementation, true);
  assert.equal(index.pages[0].evidence, null);
  const page = JSON.parse(
    fs.readFileSync(path.join(outDir, index.pages[1].evidence), 'utf8'),
  );
  assert.equal(page.sections[0].name, 'Card');
  const section = JSON.parse(
    fs.readFileSync(path.join(outDir, page.sections[0].evidence), 'utf8'),
  );
  assert.equal(section.nodes[1].text, 'Hello');
  assert.equal(index.variableCount, 2);
  assert.ok(index.values.shards.length > 0);
  assert.ok(fs.existsSync(path.join(outDir, index.values.shards[0])));

  const fullDir = fs.mkdtempSync(path.join(os.tmpdir(), 'site-spec-figma-full-'));
  const full = writeFigmaEvidence({
    outDir: fullDir,
    source: { kind: 'figma-community', fileId: '123' },
    decoded,
    byteLength: 100,
    profile: 'full',
  });
  assert.equal(full.pages[0].omittedFromImplementation, undefined);
  assert.ok(full.pages[0].evidence);
});
