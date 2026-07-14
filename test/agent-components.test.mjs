import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildAgentComponent,
  buildComponentBuildOrder,
  dedupeComponentCandidates,
  inferComponentIdentity,
  isUsefulAgentComponent,
} from '../src/agent-components.mjs';

const rootPath = 'doc(0)>main:nth-of-type(1)>section:nth-of-type(1)';
const childPath = `${rootPath}>h2:nth-of-type(1)`;
const buttonPath = `${rootPath}>button:nth-of-type(1)`;
const nodes = [
  {
    path: rootPath,
    parentPath: 'doc(0)>main:nth-of-type(1)',
    tag: 'section',
    attrs: { class: 'sc-deadbeef' },
    text: 'Next up Build a status deck',
    rect: { x: 20, y: 100, width: 500, height: 200 },
    style: { display: 'grid', gap: '16px', color: 'rgb(0, 0, 0)' },
  },
  {
    path: childPath,
    parentPath: rootPath,
    tag: 'h2',
    attrs: {},
    text: 'Next up',
    rect: { x: 20, y: 100, width: 80, height: 24 },
    style: { fontSize: '20px', fontWeight: '600' },
  },
  {
    path: buttonPath,
    parentPath: rootPath,
    tag: 'button',
    role: 'button',
    ariaLabel: 'Build a status deck',
    attrs: { 'aria-label': 'Build a status deck' },
    text: 'Build a status deck',
    rect: { x: 20, y: 140, width: 240, height: 120 },
    style: { display: 'flex', borderRadius: '12px' },
  },
];

test('emits readable compact component evidence instead of captured bundles', () => {
  const identity = inferComponentIdentity(nodes[0], nodes, 'component-001');
  const component = {
    id: 'component-001',
    identity,
    candidate: {
      representativePath: rootPath,
      occurrencePaths: [rootPath],
      reasons: ['semantic-landmark'],
    },
    captures: [{
      viewport: { width: 1440, height: 900 },
      root: nodes[0],
      nodes,
      behaviors: [{
        path: buttonPath,
        tag: 'button',
        role: 'button',
        label: 'Build a status deck',
        listeners: [{ type: 'click' }],
      }],
      exactAssets: [],
    }],
    responsive: [],
    animationImplementations: [],
  };
  const output = buildAgentComponent(component);
  const serialized = JSON.stringify(output);

  assert.equal(identity.label, 'Next up');
  assert.equal(isUsefulAgentComponent(component), true);
  assert.equal(output.structure.nodes[1].label, 'Next up');
  assert.equal(output.viewports[0].controls[0].events[0], 'click');
  assert.doesNotMatch(serialized, /site-spec-runtime|<script|base64/);
});

test('emits one component definition per repeated structural fingerprint', () => {
  const candidates = dedupeComponentCandidates([
    { node: { fingerprint: 'card' }, path: 'card-1' },
    { node: { fingerprint: 'card' }, path: 'card-2' },
    { node: { fingerprint: 'toolbar' }, path: 'toolbar' },
  ]);
  assert.deepEqual(candidates.map((candidate) => candidate.path), [
    'card-1',
    'toolbar',
  ]);
});

test('orders native component work by desktop position', () => {
  const ordered = buildComponentBuildOrder([
    { id: 'bottom', desktopRect: { x: 0, y: 400 } },
    { id: 'top-right', desktopRect: { x: 400, y: 20 } },
    { id: 'top-left', desktopRect: { x: 20, y: 20 } },
  ]);
  assert.deepEqual(ordered.map(({ id }) => id), ['top-left', 'top-right', 'bottom']);
});
