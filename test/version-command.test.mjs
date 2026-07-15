import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('prints the installed package version', () => {
  const version = execFileSync(
    process.execPath,
    [path.join(root, 'src', 'extract.mjs'), '--version'],
    { encoding: 'utf8' },
  ).trim();

  assert.equal(version, '0.1.0');
});
