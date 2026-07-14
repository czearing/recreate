import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { mergeSpecStates } from '../src/merge-spec-states.mjs';

test('merges isolated interaction states and copies their evidence', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'merge-spec-'));
  const source = path.join(root, 'source');
  const output = path.join(root, 'output');
  fs.mkdirSync(path.join(source, 'evidence'), { recursive: true });
  fs.writeFileSync(path.join(source, 'evidence', 'state-000.json'), '{"nodes":[]}');
  fs.writeFileSync(path.join(source, 'spec.json'), JSON.stringify({
    pages: [{
      index: 0,
      trigger: 'Search',
      triggerElement: { path: 'doc(0)>button:nth-of-type(1)' },
      evidence: 'evidence/state-000.json',
      evidenceByViewport: { '1440x900': 'evidence/state-000.json' },
    }],
  }));

  const states = mergeSpecStates({
    states: [{ index: -1, type: 'home' }],
    specPaths: [source],
    outDir: output,
  });

  assert.equal(states.length, 2);
  assert.equal(states[1].index, 0);
  assert.equal(states[1].trigger, 'Search');
  assert.equal(fs.existsSync(path.join(output, states[1].evidence)), true);
});

test('does not duplicate a trigger path already captured by the base spec', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'merge-spec-'));
  fs.writeFileSync(path.join(root, 'spec.json'), JSON.stringify({
    pages: [{
      index: 0,
      triggerElement: { path: 'doc(0)>button:nth-of-type(1)' },
    }],
  }));
  const states = mergeSpecStates({
    states: [{
      index: 0,
      triggerElement: { path: 'doc(0)>button:nth-of-type(1)' },
    }],
    specPaths: [root],
    outDir: path.join(root, 'output'),
  });
  assert.equal(states.length, 1);
});
