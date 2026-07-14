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
    componentCells: [{ id: 'component-card' }],
  }));
  fs.writeFileSync(path.join(root, 'report.json'), JSON.stringify({
    passed: true,
    reconstructionDetected: false,
    states: { required: 1, passed: 1, failed: 0 },
    interactions: { required: 1, passed: 1, failed: 0 },
    components: { required: 1, passed: 1, failed: 0 },
    geometry: { tolerancePx: 1, maxDeltaPx: 0.5 },
    nativeComparison: {
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
