import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { writeFigmaSection } from '../src/figma-sections.mjs';

test('preserves parent nodes when large sections split recursively', () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'site-spec-sections-'));
  const root = { id: 'root', name: 'Library', type: 'FRAME' };
  const left = { id: 'left', name: 'Left', type: 'FRAME' };
  const right = { id: 'right', name: 'Right', type: 'FRAME' };
  const children = new Map([['root', [left, right]]]);
  for (const branch of [left, right]) {
    let parent = branch;
    for (let index = 0; index < 1000; index += 1) {
      const child = {
        id: `${branch.id}-${index}`,
        name: `Node ${index}`,
        type: 'FRAME',
      };
      children.set(parent.id, [child]);
      parent = child;
    }
  }
  const section = writeFigmaSection({
    root,
    index: 0,
    directory,
    relativeDirectory: 'evidence',
    children,
    compact: (node) => node,
  });
  const payload = JSON.parse(
    fs.readFileSync(path.join(directory, path.basename(section.evidence)), 'utf8'),
  );
  assert.deepEqual(payload.root, root);
  assert.equal(payload.nodeCount, 2003);
  assert.equal(payload.sections.length, 2);
});
