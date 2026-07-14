import assert from 'node:assert/strict';
import test from 'node:test';

import { buildAgentReadiness } from '../src/agent-readiness.mjs';

test('passes only bounded complete externally indexed evidence', () => {
  const ready = buildAgentReadiness({
    implementationBytes: 7000,
    generationReady: true,
    maxComponentNodeCount: 80,
    stateIndexExternal: true,
  });
  assert.equal(ready.ready, true);

  const oversized = buildAgentReadiness({
    implementationBytes: 20000,
    generationReady: true,
    maxComponentNodeCount: 200,
    stateIndexExternal: false,
  });
  assert.equal(oversized.ready, false);
  assert.equal(oversized.failures.length, 3);
});
