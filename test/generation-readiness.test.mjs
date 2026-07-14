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
    globalPaths: [],
  });
  assert.equal(readiness.ready, false);
  assert.match(readiness.failures.join(' '), /lack readable shards/);
  assert.match(readiness.failures.join(' '), /lack captured behavior/);
});

test('passes only when complete evidence is owned by readable components', () => {
  const readiness = buildGenerationReadiness({
    capture: {
      nodes: [{ path: textPath, tag: '#text', text: 'Hero', visible: true }],
      behaviors: [{ path: `${root}>button:nth-of-type(1)`, label: 'Search', tag: 'button' }],
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
    states: [{
      index: -1,
    }, {
      index: 0,
      triggerElement: { path: `${root}>button:nth-of-type(1)` },
    }],
    viewports: [{ width: 1440 }, { width: 390 }],
    crawlRequested: true,
    globalPaths: [],
  });
  assert.equal(readiness.ready, true);
});

test('fails readiness when an enabled control has no captured behavior', () => {
  const buttonPath = `${root}>button:nth-of-type(1)`;
  const readiness = buildGenerationReadiness({
    capture: {
      nodes: [],
      behaviors: [{ path: buttonPath, label: 'Search', tag: 'button' }],
      exactAssets: [],
      animations: [],
      animationElements: [],
      lifecycleAnimation: { tracks: [] },
    },
    components: [{
      id: 'component-001',
      path: root,
      file: 'components/component-001.json',
      identity: { label: 'main' },
      nodeCounts: [1],
    }],
    states: [{ index: -1 }],
    viewports: [{ width: 1440 }, { width: 390 }],
    crawlRequested: false,
    globalPaths: [],
  });
  assert.equal(readiness.ready, false);
  assert.equal(readiness.coverage.interactions.required, 1);
  assert.equal(readiness.coverage.interactions.covered, 0);
  assert.match(readiness.failures.join(' '), /lack captured behavior/);
});

test('does not require behavior for hidden or clipped controls', () => {
  const buttonPath = `${root}>button:nth-of-type(1)`;
  const readiness = buildGenerationReadiness({
    capture: {
      nodes: [{
        path: buttonPath,
        visible: true,
        rect: { width: 2, height: 32 },
      }],
      behaviors: [{ path: buttonPath, label: 'Clipped', tag: 'button' }],
      exactAssets: [],
      animations: [],
      animationElements: [],
      lifecycleAnimation: { tracks: [] },
    },
    components: [{
      id: 'component-001',
      path: root,
      file: 'components/component-001.json',
      identity: { label: 'main' },
      nodeCounts: [1],
    }],
    states: [{ index: -1 }],
    viewports: [{ width: 1440 }, { width: 390 }],
    crawlRequested: false,
    globalPaths: [],
  });
  assert.equal(readiness.ready, true);
  assert.equal(readiness.coverage.interactions.required, 0);
});
