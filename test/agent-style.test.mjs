import assert from 'node:assert/strict';
import test from 'node:test';

import { pickAgentStyle, styleDelta } from '../src/agent-style.mjs';

test('normalizes authored kebab-case styles and preserves text typography', () => {
  assert.deepEqual(pickAgentStyle({
    'background-color': 'rgb(1, 2, 3)',
    'border-radius': '12px',
    'font-size': '20px',
    'font-weight': '600',
  }), {
    backgroundColor: 'rgb(1, 2, 3)',
    borderRadius: '12px',
    fontSize: '20px',
    fontWeight: '600',
  });
  assert.deepEqual(
    styleDelta({ fontSize: '20px' }, { fontSize: '20px' }, true),
    { fontSize: '20px' },
  );
});
