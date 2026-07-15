import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const latestReleaseUrl =
  'https://api.github.com/repos/czearing/recreate/releases/latest';

export function selectReleaseAsset(release) {
  const expected = `recreate-cli-${String(release.tag_name || '')
    .replace(/^recreate-cli-v/, '')}.tgz`;
  return (
    release.assets?.find((asset) => asset.name === expected) ||
    release.assets?.find((asset) => asset.name === 'recreate-cli.tgz')
  );
}

export function npmInvocation() {
  const candidates = [
    process.env.npm_execpath,
    path.join(
      path.dirname(process.execPath),
      'node_modules',
      'npm',
      'bin',
      'npm-cli.js',
    ),
  ].filter(Boolean);
  const cli = candidates.find((candidate) => fs.existsSync(candidate));
  return cli
    ? { command: process.execPath, prefixArgs: [cli] }
    : { command: 'npm', prefixArgs: [] };
}

export async function runLatestRecreate(
  args = process.argv.slice(2),
  fetchImpl = fetch,
  spawnImpl = spawnSync,
) {
  const response = await fetchImpl(latestReleaseUrl, {
    headers: {
      Accept: 'application/vnd.github+json',
      'User-Agent': 'recreate-cli',
    },
  });
  if (!response.ok) {
    throw new Error(`GitHub release lookup failed: HTTP ${response.status}`);
  }
  const release = await response.json();
  const asset = selectReleaseAsset(release);
  if (!asset?.browser_download_url) {
    throw new Error(`GitHub release ${release.tag_name} has no Recreate package`);
  }

  const npm = npmInvocation();
  const result = spawnImpl(
    npm.command,
    [
      ...npm.prefixArgs,
      'exec',
      '--yes',
      `--package=${asset.browser_download_url}`,
      '--',
      'recreate',
      ...args,
    ],
    { stdio: 'inherit' },
  );
  if (result.error) throw result.error;
  return result.status ?? 1;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  process.exitCode = await runLatestRecreate();
}
