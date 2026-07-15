import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('prints the installed package version', () => {
  const expectedVersion = JSON.parse(
    fs.readFileSync(path.join(root, 'package.json'), 'utf8'),
  ).version;
  const version = execFileSync(
    process.execPath,
    [path.join(root, 'src', 'extract.mjs'), '--version'],
    { encoding: 'utf8' },
  ).trim();

  assert.equal(version, expectedVersion);
});
