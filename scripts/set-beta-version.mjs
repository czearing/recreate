import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { getBetaVersion } from '../src/release-version.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const packagePath = path.join(root, 'package.json');
const lockPath = path.join(root, 'package-lock.json');
const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'));
const lockJson = JSON.parse(fs.readFileSync(lockPath, 'utf8'));
const version = getBetaVersion(
  packageJson.version,
  process.env.GITHUB_RUN_NUMBER || process.argv[2],
  process.env.GITHUB_RUN_ATTEMPT || process.argv[3],
  process.env.GITHUB_SHA || process.argv[4],
);

packageJson.version = version;
lockJson.version = version;
lockJson.packages[''].version = version;
fs.writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);
fs.writeFileSync(lockPath, `${JSON.stringify(lockJson, null, 2)}\n`);

if (process.env.GITHUB_OUTPUT) {
  fs.appendFileSync(process.env.GITHUB_OUTPUT, `version=${version}\n`);
}
console.log(version);
