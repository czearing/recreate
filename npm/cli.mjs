#!/usr/bin/env node
import { chmod, mkdir, rename, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';

const RELEASE_API =
  'https://api.github.com/repos/czearing/recreate/releases/tags/recreate-main';

export function assetName(platform = process.platform, arch = process.arch) {
  const assets = {
    'win32:x64': 'recreate-windows-x86_64.exe',
    'linux:x64': 'recreate-linux-x86_64',
    'darwin:arm64': 'recreate-macos-aarch64',
  };
  const asset = assets[`${platform}:${arch}`];
  if (!asset) throw new Error(`Unsupported platform: ${platform}-${arch}`);
  return asset;
}

export function installPath(platform = process.platform) {
  const root = process.env.RECREATE_INSTALL_DIR || join(homedir(), '.recreate', 'bin');
  return join(root, platform === 'win32' ? 'recreate.exe' : 'recreate');
}

async function downloadBinary(destination) {
  const release = await fetch(RELEASE_API, {
    headers: {
      Accept: 'application/vnd.github+json',
      'User-Agent': 'recreate-cli',
    },
  });
  if (!release.ok) throw new Error(`GitHub release lookup failed: ${release.status}`);
  const metadata = await release.json();
  const asset = metadata.assets.find(value => value.name === assetName());
  if (!asset) throw new Error(`Release asset missing: ${assetName()}`);
  const response = await fetch(asset.browser_download_url, {
    headers: { 'User-Agent': 'recreate-cli' },
  });
  if (!response.ok) throw new Error(`Binary download failed: ${response.status}`);
  const contents = Buffer.from(await response.arrayBuffer());
  const digest = `sha256:${createHash('sha256').update(contents).digest('hex')}`;
  if (asset.digest && asset.digest !== digest) {
    throw new Error(`Binary digest mismatch for ${asset.name}`);
  }
  const temporary = `${destination}.${process.pid}.tmp`;
  await mkdir(dirname(destination), { recursive: true });
  await writeFile(temporary, contents);
  if (process.platform !== 'win32') await chmod(temporary, 0o755);
  await rm(destination, { force: true });
  await rename(temporary, destination);
}

export async function run(args = process.argv.slice(2)) {
  const binary = process.env.RECREATE_BINARY || installPath();
  if (!process.env.RECREATE_BINARY) {
    try {
      const result = spawnSync(binary, args, { stdio: 'inherit' });
      if (result.status !== null) return result.status;
    } catch {}
    await downloadBinary(binary);
  }
  const result = spawnSync(binary, args, { stdio: 'inherit' });
  if (result.error) throw result.error;
  return result.status ?? 1;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  run().then(code => {
    process.exitCode = code;
  }).catch(error => {
    console.error(`recreate: ${error.message}`);
    process.exitCode = 1;
  });
}
