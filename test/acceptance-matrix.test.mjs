import assert from 'node:assert/strict';
import test from 'node:test';

import { buildAcceptanceMatrix } from '../src/acceptance-matrix.mjs';

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
    ],
  });

  assert.equal(matrix.stateCells.length, 4);
  assert.equal(matrix.interactionCells.length, 4);
  assert.equal(matrix.interactionCells.filter((cell) => cell.captured).length, 2);
  assert.equal(matrix.interactionCells.filter((cell) => !cell.captured).length, 2);
  assert.equal(matrix.componentCells[0].label, 'Notebook card');
  assert.equal(matrix.stateCells[3].evidence, 'panel-mobile.json');
  assert.match(matrix.purpose, /before PR/);
});
