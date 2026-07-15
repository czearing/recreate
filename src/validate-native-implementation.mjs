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

const compareStateFiles = (referencePath, candidatePath) => compareNativeState(
  JSON.parse(fs.readFileSync(referencePath, 'utf8')),
  JSON.parse(fs.readFileSync(candidatePath, 'utf8')),
);
const aggregateComparisons = (comparisons) => {
  const stateResults = comparisons.map(({ id, tolerancePx, result }) => ({
    id,
    passed:
      result.missing.length === 0 &&
      result.paint.mismatched === 0 &&
      result.painted.maxDeltaPx <= tolerancePx,
    required: result.required,
    matched: result.matched,
    missing: result.missing,
    maxDeltaPx: result.painted.maxDeltaPx,
    paintCompared: result.paint.compared,
    paintMismatched: result.paint.mismatched,
  }));
  return {
    stateCount: stateResults.length,
    stateIds: stateResults.map(({ id }) => id),
    statesFailed: stateResults.filter(({ passed }) => !passed).length,
    states: stateResults,
    required: stateResults.reduce((total, state) => total + state.required, 0),
    matched: stateResults.reduce((total, state) => total + state.matched, 0),
    missing: stateResults.flatMap((state) =>
      state.missing.map((identity) => `${state.id}:${identity}`)),
    painted: {
      maxDeltaPx: stateResults.length
        ? Math.max(...stateResults.map(({ maxDeltaPx }) => maxDeltaPx ?? Infinity))
        : null,
    },
    paint: {
      compared: stateResults.reduce((total, state) => total + state.paintCompared, 0),
      mismatched: stateResults.reduce((total, state) => total + state.paintMismatched, 0),
    },
  };
};

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
  { pattern: /recreate-runtime\.js/i, label: 'recreate runtime shipping' },
  { pattern: /site-spec-runtime\.js/i, label: 'legacy recreate runtime shipping' },
  { pattern: /recreate-manifest\.json/i, label: 'reconstruction manifest shipping' },
  { pattern: /site-spec-manifest\.json/i, label: 'legacy reconstruction manifest shipping' },
  { pattern: /ORACLE_ONLY\.txt/i, label: 'oracle output shipping' },
  { pattern: /__recreate[\\/]/i, label: 'reconstruction route shipping' },
  { pattern: /__site-spec[\\/]/i, label: 'legacy reconstruction route shipping' },
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
    const comparisons = [];
    if (args.comparisons) {
      const manifestPath = path.resolve(String(args.comparisons));
      const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
      const entries = Array.isArray(manifest) ? manifest : manifest.states;
      if (!Array.isArray(entries) || entries.length === 0) {
        errors.push('native comparison manifest has no states');
      } else {
        for (const entry of entries) {
          const base = path.dirname(manifestPath);
          comparisons.push({
            id: String(entry.id || ''),
            tolerancePx: Number(entry.tolerancePx || args.tolerance || 1),
            result: compareStateFiles(
              path.resolve(base, String(entry.reference || '')),
              path.resolve(base, String(entry.candidate || '')),
            ),
          });
        }
      }
    } else if (args.reference || args.candidate) {
      if (!args.reference || !args.candidate) {
        errors.push('native comparison requires both --reference and --candidate');
      } else {
        comparisons.push({
          id: String(args['state-id'] || acceptanceMatrix?.stateCells?.[0]?.id || 'state-0'),
          tolerancePx: Number(args.tolerance || acceptanceReport.geometry?.tolerancePx || 1),
          result: compareStateFiles(
            path.resolve(String(args.reference)),
            path.resolve(String(args.candidate)),
          ),
        });
      }
    }
    if (comparisons.length) {
      acceptanceReport.nativeComparison = aggregateComparisons(comparisons);
      acceptanceReport.geometry = {
        tolerancePx: Number(args.tolerance || acceptanceReport.geometry?.tolerancePx || 1),
        maxDeltaPx: acceptanceReport.nativeComparison.painted.maxDeltaPx,
      };
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
      const requiredStateIds = (acceptanceMatrix?.stateCells || [])
        .map(({ id }) => String(id))
        .sort();
      const comparedStateIds = (nativeComparison.stateIds || []).map(String).sort();
      if (
        requiredStateIds.length !== comparedStateIds.length ||
        requiredStateIds.some((id, index) => id !== comparedStateIds[index])
      ) {
        errors.push('structured acceptance report has incomplete native state comparison');
      }
      if ((nativeComparison.statesFailed ?? 0) !== 0) {
        errors.push('structured acceptance report has failed native state comparisons');
      }
    }
    const coverage = [
      ['states', acceptanceMatrix?.stateCells?.length],
      ['interactions', acceptanceMatrix?.interactionCells?.length],
      ['animations', acceptanceMatrix?.animationCells?.length],
      ['assets', acceptanceMatrix?.assetCells?.length],
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
