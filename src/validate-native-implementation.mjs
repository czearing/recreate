#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { compareNativeState } from './compare-native-state.mjs';

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
  { pattern: /site-spec-manifest\.json/i, label: 'reconstruction manifest shipping' },
  { pattern: /ORACLE_ONLY\.txt/i, label: 'oracle output shipping' },
  { pattern: /__site-spec[\\/]/i, label: 'reconstruction route shipping' },
  { pattern: /dangerouslySetInnerHTML/i, label: 'captured HTML embedding' },
];
for (const { pattern, label } of forbidden) {
  const match = sources.find(({ text }) => pattern.test(text));
  if (match) errors.push(`${label}: ${path.relative(root, match.file)}`);
}
let acceptanceReport = null;
let acceptanceMatrix = null;
if (!args.matrix) {
  errors.push('missing acceptance matrix: pass --matrix <acceptance-matrix.json>');
} else {
  try {
    acceptanceMatrix = JSON.parse(
      fs.readFileSync(path.resolve(String(args.matrix)), 'utf8'),
    );
  } catch (error) {
    errors.push(`invalid acceptance matrix: ${error.message}`);
  }
}
if (!args.report) {
  errors.push('missing structured acceptance report: pass --report <report.json>');
} else {
  const reportPath = path.resolve(root, String(args.report));
  try {
    acceptanceReport = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
    if (args.reference || args.candidate) {
      if (!args.reference || !args.candidate) {
        errors.push('native comparison requires both --reference and --candidate');
      } else {
        const nativeComparison = compareNativeState(
          JSON.parse(fs.readFileSync(path.resolve(String(args.reference)), 'utf8')),
          JSON.parse(fs.readFileSync(path.resolve(String(args.candidate)), 'utf8')),
        );
        acceptanceReport.nativeComparison = nativeComparison;
        acceptanceReport.geometry = {
          tolerancePx: Number(args.tolerance || acceptanceReport.geometry?.tolerancePx || 1),
          maxDeltaPx: nativeComparison.painted.maxDeltaPx,
        };
      }
    }
    if (acceptanceReport.passed !== true) {
      errors.push('structured acceptance report did not pass');
    }
    if (acceptanceReport.reconstructionDetected !== false) {
      errors.push('structured acceptance report did not exclude reconstruction delivery');
    }
    if ((acceptanceReport.states?.failed ?? 1) !== 0) {
      errors.push('structured acceptance report has failed states');
    }
    if ((acceptanceReport.interactions?.failed ?? 1) !== 0) {
      errors.push('structured acceptance report has failed interactions');
    }
    if ((acceptanceReport.geometry?.maxDeltaPx ?? Infinity) >
      (acceptanceReport.geometry?.tolerancePx ?? 1)) {
      errors.push('structured acceptance report exceeds geometry tolerance');
    }
    const nativeComparison = acceptanceReport.nativeComparison;
    if (!nativeComparison) {
      errors.push('structured acceptance report is missing native comparison');
    } else {
      if (
        nativeComparison.required !== nativeComparison.matched ||
        (nativeComparison.missing?.length ?? 1) !== 0
      ) {
        errors.push('structured acceptance report has incomplete native identity coverage');
      }
      if ((nativeComparison.paint?.compared ?? 0) === 0) {
        errors.push('structured acceptance report did not compare native paint');
      }
      if ((nativeComparison.paint?.mismatched ?? 1) !== 0) {
        errors.push('structured acceptance report has native paint mismatches');
      }
    }
    const coverage = [
      ['states', acceptanceMatrix?.stateCells?.length],
      ['interactions', acceptanceMatrix?.interactionCells?.length],
      ['components', acceptanceMatrix?.componentCells?.length],
    ];
    for (const [key, required] of coverage) {
      if (required == null) continue;
      if (acceptanceReport[key]?.required !== required ||
          acceptanceReport[key]?.passed !== required) {
        errors.push(`structured acceptance report has incomplete ${key} coverage`);
      }
    }
  } catch (error) {
    errors.push(`invalid structured acceptance report: ${error.message}`);
  }
}
const result = {
  passed: errors.length === 0,
  fileCount: files.length,
  required,
  sourceRoots: sourceRoots.map((value) => path.relative(root, value) || '.'),
  acceptanceReport: acceptanceReport ? String(args.report) : null,
  acceptanceMatrix: acceptanceMatrix ? String(args.matrix) : null,
  nativeComparison: acceptanceReport?.nativeComparison
    ? {
      required: acceptanceReport.nativeComparison.required,
      matched: acceptanceReport.nativeComparison.matched,
      paintMismatched: acceptanceReport.nativeComparison.paint?.mismatched,
    }
    : null,
  errors,
};
console.log(JSON.stringify(result, null, 2));
if (errors.length) process.exitCode = 1;
