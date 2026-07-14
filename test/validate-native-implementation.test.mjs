import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

const validator = path.resolve(
  import.meta.dirname,
  '..',
  'src',
  'validate-native-implementation.mjs',
);

test('native source cannot pass without structured fidelity evidence', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'native-validation-'));
  fs.mkdirSync(path.join(root, 'src'));
  fs.writeFileSync(
    path.join(root, 'src', 'app.tsx'),
    "import '@1js/bebop-icons'; import '@1js/fluentui-modern';",
  );
  const invoke = (extra = []) => spawnSync(
    process.execPath,
    [validator, '--root', root, '--paths', 'src',
      '--require', '@1js/bebop-icons,@1js/fluentui-modern', ...extra],
    { encoding: 'utf8' },
  );

  const missing = invoke();
  assert.equal(missing.status, 1);
  assert.match(missing.stdout, /missing structured acceptance report/);

  fs.writeFileSync(path.join(root, 'matrix.json'), JSON.stringify({
    stateCells: [{ id: 'state-home' }],
    interactionCells: [{ id: 'interaction-open' }],
    animationCells: [{ id: 'animation-hover' }],
    assetCells: [{ id: 'asset-icon' }],
    componentCells: [{ id: 'component-card' }],
  }));
  fs.writeFileSync(path.join(root, 'report.json'), JSON.stringify({
    passed: true,
    reconstructionDetected: false,
    states: { required: 1, passed: 1, failed: 0 },
    interactions: { required: 1, passed: 1, failed: 0 },
    animations: { required: 1, passed: 1, failed: 0 },
    assets: { required: 1, passed: 1, failed: 0 },
    components: { required: 1, passed: 1, failed: 0 },
    geometry: { tolerancePx: 1, maxDeltaPx: 0.5 },
    nativeComparison: {
      stateCount: 1,
      stateIds: ['state-home'],
      statesFailed: 0,
      required: 3,
      matched: 3,
      missing: [],
      paint: { compared: 3, mismatched: 0 },
    },
  }));
  const evidence = ['--matrix', path.join(root, 'matrix.json'), '--report', 'report.json'];
  const passed = invoke(evidence);
  assert.equal(passed.status, 0);
  assert.equal(JSON.parse(passed.stdout).passed, true);

  const reportPath = path.join(root, 'report.json');
  const referencePath = path.join(root, 'reference.json');
  const candidatePath = path.join(root, 'candidate.json');
  const comparisonState = {
    nodes: [{
      tag: 'button',
      path: 'open',
      text: 'Open',
      rect: { x: 0, y: 0, width: 32, height: 32 },
      style: { display: 'block', opacity: '1', backgroundColor: 'rgb(255, 255, 255)' },
    }],
  };
  fs.writeFileSync(referencePath, JSON.stringify(comparisonState));
  fs.writeFileSync(candidatePath, JSON.stringify(comparisonState));
  const reportWithoutComparison = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
  delete reportWithoutComparison.nativeComparison;
  fs.writeFileSync(reportPath, JSON.stringify(reportWithoutComparison));
  const directComparison = invoke([
    ...evidence,
    '--reference', referencePath,
    '--candidate', candidatePath,
  ]);
  assert.equal(directComparison.status, 0);
  assert.equal(JSON.parse(directComparison.stdout).nativeComparison.matched, 1);

  const expandedMatrix = JSON.parse(fs.readFileSync(path.join(root, 'matrix.json'), 'utf8'));
  expandedMatrix.stateCells.push({ id: 'state-mobile' });
  fs.writeFileSync(path.join(root, 'matrix.json'), JSON.stringify(expandedMatrix));
  reportWithoutComparison.states = { required: 2, passed: 2, failed: 0 };
  fs.writeFileSync(reportPath, JSON.stringify(reportWithoutComparison));
  const incompleteStates = invoke([
    ...evidence,
    '--reference', referencePath,
    '--candidate', candidatePath,
  ]);
  assert.equal(incompleteStates.status, 1);
  assert.match(incompleteStates.stdout, /incomplete native state comparison/);

  const comparisonsPath = path.join(root, 'comparisons.json');
  fs.writeFileSync(comparisonsPath, JSON.stringify({
    states: [
      { id: 'state-home', reference: 'reference.json', candidate: 'candidate.json' },
      { id: 'state-mobile', reference: 'reference.json', candidate: 'candidate.json' },
    ],
  }));
  const completeStates = invoke([...evidence, '--comparisons', comparisonsPath]);
  assert.equal(completeStates.status, 0);
  assert.equal(JSON.parse(completeStates.stdout).nativeComparison.required, 2);

  expandedMatrix.stateCells.pop();
  fs.writeFileSync(path.join(root, 'matrix.json'), JSON.stringify(expandedMatrix));
  reportWithoutComparison.states = { required: 1, passed: 1, failed: 0 };
  reportWithoutComparison.nativeComparison = {
    stateCount: 1,
    stateIds: ['state-home'],
    statesFailed: 0,
    required: 3,
    matched: 3,
    missing: [],
    paint: { compared: 3, mismatched: 0 },
  };
  fs.writeFileSync(reportPath, JSON.stringify(reportWithoutComparison));
  const failedPaint = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
  failedPaint.nativeComparison.paint.mismatched = 1;
  fs.writeFileSync(reportPath, JSON.stringify(failedPaint));
  const paintMismatch = invoke(evidence);
  assert.equal(paintMismatch.status, 1);
  assert.match(paintMismatch.stdout, /native paint mismatches/);
  failedPaint.nativeComparison.paint.mismatched = 0;
  fs.writeFileSync(reportPath, JSON.stringify(failedPaint));

  fs.appendFileSync(
    path.join(root, 'src', 'app.tsx'),
    "\nexport const route = '/__site-spec/state/000';",
  );
  const reconstruction = invoke(evidence);
  assert.equal(reconstruction.status, 1);
  assert.match(reconstruction.stdout, /reconstruction route shipping/);
});
