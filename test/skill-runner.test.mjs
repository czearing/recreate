import assert from 'node:assert/strict';
import test from 'node:test';
import {
  npmInvocation,
  runLatestRecreate,
  selectReleaseAsset,
} from '../src/skill-runner.mjs';

const release = {
  tag_name: 'recreate-cli-v1.2.3',
  assets: [
    {
      name: 'recreate-cli-1.2.3.tgz',
      browser_download_url: 'https://example.com/recreate-cli-1.2.3.tgz',
    },
  ],
};

test('selects the versioned package from the latest GitHub release', () => {
  assert.equal(
    selectReleaseAsset(release).browser_download_url,
    'https://example.com/recreate-cli-1.2.3.tgz',
  );
});

test('executes the GitHub release package without a registry lookup', async () => {
  let invocation;
  const status = await runLatestRecreate(
    ['skill'],
    async () => ({
      ok: true,
      json: async () => release,
    }),
    (command, args, options) => {
      invocation = { command, args, options };
      return { status: 0 };
    },
  );
  assert.equal(status, 0);
  const prefixLength = npmInvocation().prefixArgs.length;
  assert.equal(invocation.args[prefixLength], 'exec');
  assert.match(
    invocation.args[prefixLength + 2],
    /^--package=https:\/\/example\.com\//,
  );
  assert.deepEqual(invocation.args.slice(-3), ['--', 'recreate', 'skill']);
  assert.equal(invocation.options.stdio, 'inherit');
});
