#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const args = Object.fromEntries(process.argv.slice(2).map((arg, index, all) => [
  arg.replace(/^--/, ''),
  all[index + 1] && !all[index + 1].startsWith('--') ? all[index + 1] : true,
]));
const root = path.resolve(String(args.root || ''));
const sourceRoots = String(args.paths || '.')
  .split(',')
  .map((value) => path.resolve(root, value.trim()))
  .filter((value) => value && fs.existsSync(value));
const required = String(args.require || '')
  .split(',')
  .map((value) => value.trim())
  .filter(Boolean);
if (!args.root) throw new Error('Pass --root <implementation-root>.');
if (!sourceRoots.length) throw new Error('No implementation source roots exist.');

const sourceExtensions = new Set(['.ts', '.tsx', '.js', '.jsx', '.mjs']);
const files = [];
const visit = (directory) => {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (['node_modules', 'dist', 'lib', 'coverage'].includes(entry.name)) continue;
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) visit(file);
    else if (sourceExtensions.has(path.extname(entry.name))) files.push(file);
  }
};
for (const sourceRoot of sourceRoots) visit(sourceRoot);
const sources = files.map((file) => ({
  file,
  text: fs.readFileSync(file, 'utf8'),
}));
const errors = [];
for (const dependency of required) {
  if (!sources.some(({ text }) => text.includes(dependency))) {
    errors.push(`missing required native dependency import: ${dependency}`);
  }
}
const forbidden = [
  { pattern: /<iframe\b/i, label: 'iframe embedding' },
  { pattern: /site-spec-runtime\.js/i, label: 'site-spec runtime shipping' },
  { pattern: /new-office-spec\/index\.html/i, label: 'reconstruction redirect' },
  { pattern: /dangerouslySetInnerHTML/i, label: 'captured HTML embedding' },
];
for (const { pattern, label } of forbidden) {
  const match = sources.find(({ text }) => pattern.test(text));
  if (match) errors.push(`${label}: ${path.relative(root, match.file)}`);
}
const result = {
  passed: errors.length === 0,
  fileCount: files.length,
  required,
  sourceRoots: sourceRoots.map((value) => path.relative(root, value) || '.'),
  errors,
};
console.log(JSON.stringify(result, null, 2));
if (errors.length) process.exitCode = 1;
