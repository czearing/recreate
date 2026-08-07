// Scores the LOCKED pre-registered rule against measured outcomes.
// Reads the prereg file for labels; never writes to its preRegistration block.
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const M = process.cwd();
const DIR = join(M, 'wf');
const PREALL = JSON.parse(readFileSync(join(M, 'width-fidelity-prereg.json'), 'utf8'));
const PRE = PREALL.preRegistration;
const LABELS = { ...PRE.predictedLabels, ...PREALL.preRegistrationBatch2.predictedLabels };
const W = PRE.decisionRule.probeWidths;
const TOL = PRE.decisionRule.tolerancePx;
const SITES = ['reactdev', 'sveltedev', 'vuejs', 'cern', 'danluu', 'nprtext', 'lobsters', 'gnu', 'w3c', 'sourcehut'];

const boxMap = (arr) => {
  const m = new Map();
  for (const [t, l, w] of arr || []) if (!m.has(t)) m.set(t, [l, w]);
  return m;
};

const pages = {};
for (const s of SITES) {
  const j = JSON.parse(readFileSync(join(DIR, `raw-${s}.json`), 'utf8'));
  const rec = {}, src = {}, delta = {}, exposure = {};

  for (const w of W) {
    const p = j.perWidth[w];
    rec[w] = p.recreationOnly;
    src[w] = p.sourceOnly;

    const S = boxMap(p.boxes.source), R = boxMap(p.boxes.recreation);
    let shared = 0, mismatch = 0, worst = 0, worstText = null;
    for (const [t, [, sw]] of S) {
      const r = R.get(t); if (!r) continue;
      shared++;
      const d = Math.abs(r[1] - sw);
      if (d > TOL) mismatch++;
      if (d > worst) { worst = d; worstText = t; }
    }
    delta[w] = {
      sharedTexts: shared, widthMismatched: mismatch,
      widthMismatchPct: shared ? +(100 * mismatch / shared).toFixed(1) : null,
      maxWidthDeltaPx: worst, maxWidthDeltaText: worstText,
    };

    // exposure: distinct texts behind the pairs, not just pair totals
    const texts = new Set();
    for (const ex of [...(p.recreationOnlyExamples || []), ...(p.sourceOnlyExamples || [])]) {
      for (const t of (Array.isArray(ex) ? ex : [ex.a, ex.b])) if (t) texts.add(String(t));
    }
    exposure[w] = {
      recreationOnlyPairs: p.recreationOnly, sourceOnlyPairs: p.sourceOnly,
      distinctTextsInExamples: texts.size,
      sourceCandidates: p.source.candidates, recreationCandidates: p.recreation.candidates,
      examplesCapped: 8,
    };
  }

  // reflow control between the extreme widths
  const reflow = (side) => {
    const a = boxMap(j.perWidth[W[0]].boxes[side]), b = boxMap(j.perWidth[W[W.length - 1]].boxes[side]);
    let n = 0, moved = 0;
    for (const [t, [l, wd]] of a) {
      const o = b.get(t); if (!o) continue;
      n++; if (Math.abs(o[0] - l) > TOL || Math.abs(o[1] - wd) > TOL) moved++;
    }
    return { compared: n, movedPct: n ? +(100 * moved / n).toFixed(1) : null };
  };
  const rfS = reflow('source'), rfR = reflow('recreation');

  const capMax = Math.max(...W.map((w) => j.perWidth[w].inlineRowCapacity.source.maxRowMembers));
  const anyPairs = W.some((w) => j.perWidth[w].source.pairs > 0 || j.perWidth[w].recreation.pairs > 0);
  const totalRec = W.reduce((a, w) => a + rec[w], 0);
  const totalSrc = W.reduce((a, w) => a + src[w], 0);

  // Grading, inherited unchanged from the previous run.
  const capable = capMax > 0;
  const reflows = rfS.movedPct > 0 && rfR.movedPct > 0;
  let grade, why;
  if (!capable) { grade = 'weak'; why = 'inline-row capacity 0 at every width: structurally incapable of overlap.'; }
  else if (!reflows) { grade = 'weak'; why = `does not reflow between ${W[0]} and ${W[W.length - 1]} on both sides (source ${rfS.movedPct}%, recreation ${rfR.movedPct}%): width-dependent divergence cannot be observed.`; }
  else if (totalRec + totalSrc > 0) { grade = 'informative-positive'; why = 'capable, reflows, and divergence was recorded.'; }
  else if (anyPairs) { grade = 'informative-negative'; why = 'capable, reflows, predicate fired, yet neither side diverged: a real clean result.'; }
  else { grade = 'weak'; why = 'capable but the predicate never fired on either side at any width: the zero is instrument silence.'; }

  // The locked rule, applied.
  const nonzeroWidths = W.filter((w) => delta[w].widthMismatchPct > 0);
  const ruleLabel = nonzeroWidths.length === 0 ? 'CLEAN' : 'DIVERGENT';
  const outcome = (totalRec + totalSrc) > 0 ? 'DIVERGENT' : 'CLEAN';

  pages[s] = {
    url: LABELS[s].url, stack: LABELS[s].stack,
    predictedLabel: LABELS[s].predicted,
    recreationOnlyPairsPerWidth: rec, sourceOnlyPairsPerWidth: src,
    totals: { recreationOnly: totalRec, sourceOnly: totalSrc },
    widthFidelityPerWidth: delta,
    widthsWithNonzeroMismatch: nonzeroWidths,
    exposurePerWidth: exposure,
    reflowControl: { source: rfS, recreation: rfR },
    inlineRowCapacityMaxSource: capMax,
    grade, gradeReason: why,
    ruleSaysLabel: ruleLabel,
    measuredOutcome: outcome,
    ruleCorrect: ruleLabel === outcome,
    predictionCorrect: LABELS[s].predicted === outcome,
  };
}

const informative = SITES.filter((s) => pages[s].grade.startsWith('informative'));
const scored = informative;
const ruleHits = scored.filter((s) => pages[s].ruleCorrect);
const predHits = scored.filter((s) => pages[s].predictionCorrect);
const divergentCount = scored.filter((s) => pages[s].measuredOutcome === 'DIVERGENT').length;

const summary = {
  totalPagesRegistered: Object.keys(LABELS).length,
  pagesMeasured: SITES.length,
  informativePages: informative.length,
  informativePageList: informative,
  weakPages: SITES.filter((s) => pages[s].grade === 'weak'),
  scoredDenominator: scored.length,
  widthFidelityRule: {
    hits: `${ruleHits.length} of ${scored.length}`,
    wrongOn: scored.filter((s) => !pages[s].ruleCorrect),
  },
  mechanismPrediction: {
    hits: `${predHits.length} of ${scored.length}`,
    wrongOn: scored.filter((s) => !pages[s].predictionCorrect),
  },
  baselines: {
    alwaysDivergent: `${divergentCount} of ${scored.length}`,
    alwaysClean: `${scored.length - divergentCount} of ${scored.length}`,
  },
};

writeFileSync(join(DIR, 'wf-scored.json'), JSON.stringify({ pages, summary }, null, 2));

const hdr = 'site'.padEnd(11) + 'pred'.padEnd(11) + 'rule'.padEnd(11) + 'outcome'.padEnd(11) + 'grade'.padEnd(23) + 'rec/src';
console.log(hdr);
for (const s of SITES) {
  const p = pages[s];
  console.log(s.padEnd(11) + p.predictedLabel.padEnd(11) + p.ruleSaysLabel.padEnd(11) + p.measuredOutcome.padEnd(11) + p.grade.padEnd(23) + `${p.totals.recreationOnly}/${p.totals.sourceOnly}`);
}
console.log('\nwidth-mismatch pct per width:');
console.log('site'.padEnd(11) + W.map((w) => String(w).padStart(7)).join(''));
for (const s of SITES) console.log(s.padEnd(11) + W.map((w) => String(pages[s].widthFidelityPerWidth[w].widthMismatchPct).padStart(7)).join(''));
console.log('\nrule    ', summary.widthFidelityRule.hits, 'wrong on:', summary.widthFidelityRule.wrongOn.join(',') || 'none');
console.log('mechanism', summary.mechanismPrediction.hits, 'wrong on:', summary.mechanismPrediction.wrongOn.join(',') || 'none');
console.log('baseline always-divergent', summary.baselines.alwaysDivergent);
console.log('informative', summary.informativePages, '/', SITES.length, ' weak:', summary.weakPages.join(',') || 'none');
