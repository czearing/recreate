import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildAcceptanceMatrix,
  buildImplementationStateIndex,
} from '../src/acceptance-matrix.mjs';

test('expands every captured state and interaction across each viewport', () => {
  const matrix = buildAcceptanceMatrix({
    states: [
      { index: -1, type: 'home', url: '/app', evidence: 'home.json' },
      {
        index: 0,
        type: 'panel',
        url: '/app',
        trigger: 'Open',
        triggerElement: { path: 'doc(0)>button:nth-of-type(1)', label: 'Open', tag: 'button' },
        probe: { action: 'click' },
        evidenceByViewport: { '390x844': 'panel-mobile.json' },
      },
    ],
    viewports: [
      { width: 1440, height: 900, dpr: 1 },
      { width: 390, height: 844, dpr: 1 },
    ],
    components: [{
      id: 'component-001',
      file: 'components/component-001.json',
      identity: { label: 'Notebook card' },
    }],
    controls: [
      { path: 'doc(0)>button:nth-of-type(1)', label: 'Open', tag: 'button' },
      { path: 'doc(0)>button:nth-of-type(2)', label: 'Search', tag: 'button' },
      { path: 'doc(0)>button:nth-of-type(3)', label: 'Clipped', tag: 'button' },
    ],
    nodes: [{
      path: 'doc(0)>button:nth-of-type(3)',
      visible: true,
      rect: { width: 2, height: 32 },
    }],
    animations: [{ path: 'doc(0)>article:nth-of-type(1)', type: 'hover' }],
    assets: [{
      path: 'doc(0)>img:nth-of-type(1)',
      type: 'image',
      file: 'snapshot-assets/example.svg',
      naturalWidth: 64,
      naturalHeight: 64,
    }],
  });

  assert.equal(matrix.stateCells.length, 2);
  assert.equal(matrix.interactionCells.length, 4);
  assert.equal(matrix.interactionCells.filter((cell) => cell.captured).length, 2);
  assert.equal(matrix.interactionCells.filter((cell) => !cell.captured).length, 2);
  assert.equal(matrix.animationCells.length, 2);
  assert.equal(matrix.assetCells.length, 1);
  assert.equal(matrix.componentCells[0].label, 'Notebook card');
  assert.equal(
    matrix.interactionCells.find((cell) => cell.captured && cell.viewport.width === 390).evidence,
    'panel-mobile.json',
  );
  assert.match(matrix.purpose, /before PR/);
});

test('keeps the implementation state index small and links heavy evidence', () => {
  const [state] = buildImplementationStateIndex([{
    index: 2,
    type: 'panel',
    trigger: 'Search',
    probe: { action: 'click' },
    url: '/app',
    evidence: 'evidence/state-002.json',
    evidenceByViewport: { '390x844': 'evidence/state-002-390x844.json' },
    network: Array.from({ length: 100 }, () => ({ url: '/large' })),
    dismissal: { samples: Array.from({ length: 100 }, () => 'large') },
  }]);
  assert.equal(state.action, 'click');
  assert.equal(state.evidence, 'evidence/state-002.json');
  assert.equal('network' in state, false);
  assert.equal('dismissal' in state, false);
});
