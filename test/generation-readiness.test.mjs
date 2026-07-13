import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGenerationReadiness } from '../src/generation-readiness.mjs';

const root = 'doc(0)>main:nth-of-type(1)';
const textPath = `${root}>h1:nth-of-type(1)`;

test('fails whole-site readiness when visible regions are not readable', () => {
  const readiness = buildGenerationReadiness({
    capture: {
      nodes: [{ path: textPath, tag: '#text', text: 'Missing hero', visible: true }],
      behaviors: [{ path: `${root}>button:nth-of-type(1)`, label: 'Search', tag: 'button' }],
      exactAssets: [],
      animations: [{}],
      animationElements: [],
      lifecycleAnimation: { tracks: [] },
    },
    components: [{
      id: 'component-001',
      path: root,
      identity: { label: 'main' },
      nodeCounts: [3],
    }],
    states: [{ index: -1 }],
    viewports: [{ width: 1440 }, { width: 390 }],
    crawlRequested: true,
  });
  assert.equal(readiness.ready, false);
  assert.match(readiness.failures.join(' '), /lack readable shards/);
  assert.match(readiness.failures.join(' '), /no interaction states/);
  assert.match(readiness.failures.join(' '), /animation definition/);
});

test('passes only when complete evidence is owned by readable components', () => {
  const readiness = buildGenerationReadiness({
    capture: {
      nodes: [{ path: textPath, tag: '#text', text: 'Hero', visible: true }],
      behaviors: [{ path: `${root}>button:nth-of-type(1)`, label: 'Search' }],
      exactAssets: [{ path: `${root}>img:nth-of-type(1)`, type: 'image' }],
      animations: [{}],
      animationElements: [{ path: `${root}>div:nth-of-type(1)`, type: 'hover' }],
      lifecycleAnimation: { tracks: [] },
    },
    components: [{
      id: 'component-001',
      path: root,
      file: 'components/component-001.json',
      identity: { label: 'main' },
      nodeCounts: [5],
    }],
    states: [{ index: -1 }, { index: 0 }],
    viewports: [{ width: 1440 }, { width: 390 }],
    crawlRequested: true,
  });
  assert.equal(readiness.ready, true);
});
