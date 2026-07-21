import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';
import { assetName, installPath } from './cli.mjs';

test('maps every published platform asset', () => {
  assert.equal(assetName('win32', 'x64'), 'recreate-windows-x86_64.exe');
  assert.equal(assetName('linux', 'x64'), 'recreate-linux-x86_64');
  assert.equal(assetName('darwin', 'arm64'), 'recreate-macos-aarch64');
  assert.throws(() => assetName('darwin', 'x64'), /Unsupported platform/);
});

test('uses the shared Recreate install location', () => {
  assert.match(installPath('win32'), /[\\/]\.recreate[\\/]bin[\\/]recreate\.exe$/);
  assert.match(installPath('linux'), /[\\/]\.recreate[\\/]bin[\\/]recreate$/);
});

test('forwards arguments to an existing native executable', () => {
  const result = spawnSync(process.execPath, ['npm/cli.mjs', '--version'], {
    cwd: new URL('..', import.meta.url),
    encoding: 'utf8',
    env: { ...process.env, RECREATE_BINARY: process.execPath },
  });
  assert.equal(result.status, 0);
  assert.match(result.stdout, new RegExp(process.version.replaceAll('.', '\\.')));
});
