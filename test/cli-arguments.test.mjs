import test from 'node:test';
import assert from 'node:assert/strict';
import { getPositionalUrl } from '../src/cli-arguments.mjs';

test('uses the first positional argument as the source URL', () => {
  assert.equal(
    getPositionalUrl(['https://example.com', '--crawl']),
    'https://example.com',
  );
});

test('does not mistake a flag value for a positional URL', () => {
  assert.equal(getPositionalUrl(['--url', 'https://example.com']), '');
});
