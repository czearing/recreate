import { readFile } from 'node:fs/promises';

const manifest = JSON.parse(await readFile(new URL('../package.json', import.meta.url)));
const cargo = await readFile(new URL('../Cargo.toml', import.meta.url), 'utf8');
const version = cargo.match(/^version = "([^"]+)"/m)?.[1];

if (manifest.version !== version) {
  throw new Error(`package.json ${manifest.version} does not match Cargo.toml ${version}`);
}
if (manifest.name !== 'recreate-cli') {
  throw new Error(`unexpected package name: ${manifest.name}`);
}
