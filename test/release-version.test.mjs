import test from 'node:test';
import assert from 'node:assert/strict';
import { getBetaVersion, getStableVersion } from '../src/release-version.mjs';

test('extracts the stable semantic version', () => {
  assert.equal(getStableVersion('1.2.3-beta.4'), '1.2.3');
});

test('builds a unique ordered beta version', () => {
  assert.equal(
    getBetaVersion('0.1.0', '42', '2', 'ABCDEF0123456789'),
    '0.1.0-beta.42.2.sha-abcdef0',
  );
});

test('rejects invalid release inputs', () => {
  assert.throws(() => getStableVersion('1.2'));
  assert.throws(() => getBetaVersion('1.2.3', '0', '1', 'abcdef0'));
  assert.throws(() => getBetaVersion('1.2.3', '1', '1', 'bad'));
});
