import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { writeFigmaEvidence } from '../src/figma-evidence.mjs';

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
        styleIdForFill: { guid: { sessionID: 4, localID: 1 } },
        prototypeInteractions: [{
          id: { sessionID: 9, localID: 1 },
          event: { interactionType: 'ON_PRESS' },
          actions: [{
            transitionNodeID: { sessionID: 2, localID: 3 },
            navigationType: 'NAVIGATE',
            connectionType: 'INTERNAL_NODE',
            transitionType: 'SMART_ANIMATE',
          }],
        }, {
          id: { sessionID: 9, localID: 2 },
          event: {
            interactionType: 'AFTER_TIMEOUT',
            transitionTimeout: 0.5,
          },
          actions: [{
            transitionNodeID: { sessionID: 2, localID: 3 },
            navigationType: 'OVERLAY',
            connectionType: 'INTERNAL_NODE',
            transitionType: 'MOVE_FROM_RIGHT',
          }, {
            connectionType: 'URL',
            url: 'https://example.com',
          }],
        }],
      }),
      node(2, 3, 'TEXT', 'Title', [2, 2], {
        textData: { characters: 'Hello' },
      }),
      node(3, 1, 'VARIABLE_SET', 'Theme', [0, 0]),
      node(3, 2, 'VARIABLE', 'Color', [0, 0], {
        variableSetID: { sessionID: 3, localID: 1 },
        variableDataValues: {
          description: 'A sufficiently long exact variable payload for reference sharding.',
        },
      }),
      node(4, 1, 'ROUNDED_RECTANGLE', 'Card fill', [0, 0], {
        fillPaints: [{ type: 'SOLID', color: { r: 1, g: 0, b: 0, a: 1 } }],
      }),
    ],
  },
};

test('writes semantic indexes and omits hidden pages by default', () => {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'recreate-figma-'));
  const index = writeFigmaEvidence({
    outDir,
    source: { kind: 'figma-community', fileId: '123' },
    decoded,
    byteLength: 100,
    profile: 'implementation',
  });
  assert.equal(index.pages[0].omittedFromImplementation, true);
  const page = JSON.parse(
    fs.readFileSync(path.join(outDir, index.pages[1].evidence), 'utf8'),
  );
  const section = JSON.parse(
    fs.readFileSync(path.join(outDir, page.sections[0].evidence), 'utf8'),
  );
  assert.equal(section.nodes[1].text, 'Hello');
  assert.equal(index.variableCount, 2);
  assert.equal(index.components.count, 1);
  assert.equal(index.interactions.interactionCount, 2);
  assert.equal(index.styles.count, 1);
  assert.equal(index.styles.missingCount, 0);
  assert.ok(index.values.shards.length > 0);
  for (const file of [
    index.components.search,
    index.interactions.search,
    index.styles.file,
    index.values.shards[0],
  ]) assert.ok(fs.existsSync(path.join(outDir, file)));
  const componentShard = JSON.parse(
    fs.readFileSync(path.join(outDir, index.components.details.shards[0]), 'utf8'),
  );
  const component = Object.values(componentShard.components)[0];
  assert.equal(component.master.type, 'SYMBOL');
  assert.ok(component.masterChildren);
  const interactionSearch = JSON.parse(
    fs.readFileSync(path.join(outDir, index.interactions.search), 'utf8'),
  );
  assert.deepEqual(interactionSearch.flows[0].eventTypes, [
    'ON_PRESS',
    'AFTER_TIMEOUT',
  ]);
  assert.deepEqual(interactionSearch.facets.navigationTypes, {
    NAVIGATE: [0],
    OVERLAY: [0],
    NONE: [0],
  });
  assert.deepEqual(interactionSearch.facets.connectionTypes, {
    INTERNAL_NODE: [0],
    URL: [0],
  });
  assert.deepEqual(interactionSearch.facets.transitionTypes, {
    SMART_ANIMATE: [0],
    MOVE_FROM_RIGHT: [0],
    NONE: [0],
  });
  assert.deepEqual(interactionSearch.facets.timeoutSeconds, {
    0: [0.5],
  });
  assert.equal(interactionSearch.facets.values, 'flow indexes');
});

test('retains hidden backing pages in full profile', () => {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'recreate-figma-full-'));
  const index = writeFigmaEvidence({
    outDir,
    source: { kind: 'figma-community', fileId: '123' },
    decoded,
    byteLength: 100,
    profile: 'full',
  });
  assert.equal(index.pages[0].omittedFromImplementation, undefined);
  assert.ok(index.pages[0].evidence);
});
