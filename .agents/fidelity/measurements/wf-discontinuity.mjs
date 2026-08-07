// Discriminates the frozen-geometry signature from the rival "error just falls
// as viewports widen". Both predict the minimum at 1440, because 1440 is
// unfortunately BOTH the only sampled width in the sweep and its widest point.
// Only a DISCONTINUITY at the 1076->1440 step distinguishes them: sampling
// predicts a step change there specifically, monotone decline predicts drops
// spread across the sweep.
import fs from 'node:fs';

const WIDTHS = [400, 450, 480, 600, 661, 800, 1076, 1440];
const sig = JSON.parse(fs.readFileSync('wf/wf-signature.json', 'utf8'));

function spearman(xs, ys) {
  const rank = v => { const s = [...v].sort((a, b) => a - b); return v.map(x => (s.indexOf(x) + s.lastIndexOf(x)) / 2 + 1); };
  const [a, b] = [rank(xs), rank(ys)];
  const n = a.length, ma = a.reduce((p, c) => p + c, 0) / n, mb = b.reduce((p, c) => p + c, 0) / n;
  let num = 0, da = 0, db = 0;
  for (let i = 0; i < n; i++) { num += (a[i] - ma) * (b[i] - mb); da += (a[i] - ma) ** 2; db += (b[i] - mb) ** 2; }
  return da && db ? +(num / Math.sqrt(da * db)).toFixed(3) : null;
}

const res = {};
for (const [s, v] of Object.entries(sig)) {
  const usable = WIDTHS.filter(w => v.perWidth && v.perWidth[w]);
  if (usable.length < WIDTHS.length) { res[s] = { batch: v.batch, testable: false, reason: 'no shared texts on either side' }; continue; }
  const pct = WIDTHS.map(w => v.perWidth[w].pct);
  const flat = pct.every(p => p === pct[0]);
  if (flat) { res[s] = { batch: v.batch, testable: false, reason: `flat at ${pct[0]}% across all widths - no minimum to locate` }; continue; }

  const min = Math.min(...pct);
  const atMinAtSampled = pct[WIDTHS.length - 1] === min;
  // adjacent downward steps; index i is the step WIDTHS[i] -> WIDTHS[i+1]
  const steps = pct.slice(0, -1).map((p, i) => +(p - pct[i + 1]).toFixed(1));
  const sampledStep = steps[steps.length - 1];           // 1076 -> 1440
  const others = steps.slice(0, -1);
  const maxOther = Math.max(...others.map(Math.abs));
  const biggestStepIsAtSampled = Math.abs(sampledStep) >= maxOther && sampledStep > 0;
  const largestStepIdx = steps.reduce((bi, x, i) => Math.abs(x) > Math.abs(steps[bi]) ? i : bi, 0);

  res[s] = {
    batch: v.batch, testable: true,
    pctByWidth: Object.fromEntries(WIDTHS.map((w, i) => [w, pct[i]])),
    minPct: min, pctAtSampled: pct[7],
    atMinimumAtSampledWidth: atMinAtSampled,
    stepsDownBetweenAdjacentWidths: Object.fromEntries(steps.map((x, i) => [`${WIDTHS[i]}->${WIDTHS[i + 1]}`, x])),
    stepAtSampledBoundary: sampledStep,
    largestStepInSweep: `${WIDTHS[largestStepIdx]}->${WIDTHS[largestStepIdx + 1]}`,
    largestStepIsAtSampledBoundary: biggestStepIsAtSampled,
    spearmanPctVsWidth: spearman(WIDTHS, pct),
    monotoneNonIncreasing: pct.every((p, i) => i === 0 || p <= pct[i - 1] + 1e-9),
    // A page whose error has already collapsed by 1076 has no headroom left to
    // fall at 1440, so it can neither confirm nor refute a step there.
    // Threshold chosen AFTER seeing the data and declared as such: the observed
    // plateau floors are 0, 2.3 and 2.8, so 5.0 sits clear of all of them.
    pctAt1076: pct[6],
    poweredForStepTest: pct[6] > 5.0,
    tiedRatherThanStrict: Math.abs(sampledStep) === maxOther,
    // Second confound: 1076->1440 is also the WIDEST step in the sweep (364px vs
    // 30-276px). A change proportional to viewport travel would land its largest
    // raw step there for reasons unrelated to sampling. Normalise per pixel.
    stepsPerPx: Object.fromEntries(steps.map((x, i) => [`${WIDTHS[i]}->${WIDTHS[i + 1]}`, +(x / (WIDTHS[i + 1] - WIDTHS[i])).toFixed(5)])),
    normStepAtSampled: +(sampledStep / (1440 - 1076)).toFixed(5),
    maxNormStepElsewhere: +Math.max(...others.map((x, i) => Math.abs(x / (WIDTHS[i + 1] - WIDTHS[i])))).toFixed(5),
    largestNormStepIsAtSampledBoundary:
      sampledStep > 0 && (sampledStep / 364) >= Math.max(...others.map((x, i) => Math.abs(x / (WIDTHS[i + 1] - WIDTHS[i])))),
  };
}
fs.writeFileSync('wf/wf-discontinuity.json', JSON.stringify(res, null, 2));

const T = Object.entries(res).filter(([, v]) => v.testable);
console.log('site        b  min%  @1440  atMin?  stepAt1076->1440  largestStep   rho    monotone?');
for (const [s, v] of Object.entries(res)) {
  if (!v.testable) { console.log(s.padEnd(11) + String(v.batch).padEnd(2) + '  UNTESTABLE: ' + v.reason); continue; }
  console.log(s.padEnd(11) + String(v.batch).padEnd(2) +
    String(v.minPct).padStart(6) + String(v.pctAtSampled).padStart(7) +
    String(v.atMinimumAtSampledWidth).padStart(8) + String(v.stepAtSampledBoundary).padStart(18) +
    ('  ' + v.largestStepInSweep).padEnd(14) + String(v.spearmanPctVsWidth).padStart(7) +
    String(v.monotoneNonIncreasing).padStart(10));
}
const atMin = T.filter(([, v]) => v.atMinimumAtSampledWidth).length;
const disc = T.filter(([, v]) => v.largestStepIsAtSampledBoundary).length;
const mono = T.filter(([, v]) => v.monotoneNonIncreasing).length;
console.log(`\ntestable pages           : ${T.length}`);
console.log(`minimum sits at 1440     : ${atMin} of ${T.length}   (confounded: 1440 is also the widest probe)`);
console.log(`LARGEST step at 1076->1440: ${disc} of ${T.length}   (unconfounded discontinuity test)`);
console.log(`monotone non-increasing  : ${mono} of ${T.length}   (rival explanation holds outright)`);
console.log('failed the discontinuity test: ' + (T.filter(([, v]) => !v.largestStepIsAtSampledBoundary).map(([s]) => s).join(', ') || 'none'));

const P = T.filter(([, v]) => v.poweredForStepTest);
const pOK = P.filter(([, v]) => v.largestStepIsAtSampledBoundary);
console.log(`\n-- powered subset (error still >5%% at 1076, so a step at 1440 is possible) --`);
console.log('powered pages            : ' + P.length + ' of ' + T.length + '  [' + P.map(([s]) => s).join(', ') + ']');
console.log('underpowered (plateaued) : ' + T.filter(([, v]) => !v.poweredForStepTest).map(([s, v]) => `${s}@1076=${v.pctAt1076}%`).join(', '));
console.log('step at sampled boundary : ' + pOK.length + ' of ' + P.length + '  [' + pOK.map(([s, v]) => s + (v.tiedRatherThanStrict ? ' (tie)' : '')).join(', ') + ']');
console.log('counterexample(s)        : ' + (P.filter(([, v]) => !v.largestStepIsAtSampledBoundary)
  .map(([s, v]) => `${s} (1076=${v.pctAt1076}% -> 1440=${v.pctAtSampled}%, step ${v.stepAtSampledBoundary}, largest step ${v.largestStepInSweep})`).join('; ') || 'none'));

const nOK = P.filter(([, v]) => v.largestNormStepIsAtSampledBoundary);
console.log('\n-- span-normalised (%% per px of viewport travel; removes the widest-step confound) --');
for (const [s, v] of P) {
  console.log('  ' + s.padEnd(11) + 'at 1076->1440 ' + String(v.normStepAtSampled).padStart(8) +
    '   best elsewhere ' + String(v.maxNormStepElsewhere).padStart(8) +
    '   ' + (v.largestNormStepIsAtSampledBoundary ? 'SIGNATURE HOLDS' : 'fails'));
}
console.log('span-normalised result   : ' + nOK.length + ' of ' + P.length + '  [' + nOK.map(([s]) => s).join(', ') + ']');

// exact binomial tail against "the largest step lands on any of the 7 boundaries"
const binom = (n, k) => { let r = 1; for (let i = 0; i < k; i++) r = r * (n - i) / (i + 1); return r; };
const tail = (k, n, p) => { let s = 0; for (let i = k; i <= n; i++) s += binom(n, i) * p ** i * (1 - p) ** (n - i); return s; };
console.log(`\nnull = largest step lands on any of 7 boundaries (p=1/7)`);
console.log(`  raw-step result   ${pOK.length}/${P.length}  exact one-sided p = ${tail(pOK.length, P.length, 1 / 7).toExponential(2)}`);
console.log(`  norm-step result  ${nOK.length}/${P.length}  exact one-sided p = ${tail(nOK.length, P.length, 1 / 7).toExponential(2)}`);
