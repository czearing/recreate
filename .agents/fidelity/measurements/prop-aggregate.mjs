// Aggregates the six per-page property-census records into grades and a verdict.
// Reads only raw-*.json. Does not re-measure and does not touch the predicate.
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const DIR = join(process.cwd(), 'property');
const SITES = ['godev', 'jquery', 'litenews', 'paper', 'phpman', 'plaintext'];
const WIDTHS = [400, 450, 480, 600, 661, 800, 1076, 1440];

const boxMap = (arr) => {
  const m = new Map();
  for (const [t, l, w] of arr || []) if (!m.has(t)) m.set(t, [l, w]);
  return m;
};

const out = {};
for (const site of SITES) {
  const j = JSON.parse(readFileSync(join(DIR, `raw-${site}.json`), 'utf8'));
  const rec = {}, src = {}, cap = {}, delta = {};

  for (const w of WIDTHS) {
    const p = j.perWidth[w];
    rec[w] = p.recreationOnly;
    src[w] = p.sourceOnly;
    cap[w] = {
      source: p.inlineRowCapacity.source.maxRowMembers,
      recreation: p.inlineRowCapacity.recreation.maxRowMembers,
    };

    // width-delta covariate: same text on both sides, compare left and width
    const S = boxMap(p.boxes.source), R = boxMap(p.boxes.recreation);
    let shared = 0, mismatch = 0, worst = 0, worstText = null, leftOk = 0;
    for (const [t, [sl, sw]] of S) {
      const r = R.get(t);
      if (!r) continue;
      shared++;
      if (Math.abs(r[0] - sl) <= 2) leftOk++;
      const d = Math.abs(r[1] - sw);
      if (d > 2) mismatch++;
      if (d > worst) { worst = d; worstText = t; }
    }
    delta[w] = {
      sharedTexts: shared,
      widthMismatched: mismatch,
      widthMismatchPct: shared ? +(100 * mismatch / shared).toFixed(1) : null,
      leftAgreePct: shared ? +(100 * leftOk / shared).toFixed(1) : null,
      maxWidthDeltaPx: worst,
      maxWidthDeltaText: worstText,
    };
  }

  // reflow control: does the page's own layout change between 400 and 1440?
  const reflow = (side) => {
    const a = boxMap(j.perWidth[400].boxes[side]);
    const b = boxMap(j.perWidth[1440].boxes[side]);
    let n = 0, moved = 0;
    for (const [t, [l, w]] of a) {
      const o = b.get(t); if (!o) continue;
      n++; if (Math.abs(o[0] - l) > 2 || Math.abs(o[1] - w) > 2) moved++;
    }
    return { compared: n, movedPct: n ? +(100 * moved / n).toFixed(1) : null };
  };
  const reflowSrc = reflow('source'), reflowRec = reflow('recreation');

  const capMax = Math.max(...WIDTHS.map((w) => cap[w].source));
  const totalRec = WIDTHS.reduce((s, w) => s + rec[w], 0);
  const totalSrc = WIDTHS.reduce((s, w) => s + src[w], 0);
  const anyPairs = WIDTHS.some((w) => j.perWidth[w].source.pairs > 0 || j.perWidth[w].recreation.pairs > 0);

  // Grading. A zero is only informative if the page could have shown a defect:
  // both sides render, both sides reflow, and the page is structurally capable.
  const capable = capMax > 0;
  const reflows = reflowSrc.movedPct > 0 && reflowRec.movedPct > 0;
  let grade, why;
  if (!capable) {
    grade = 'weak';
    why = 'inline-row capacity is 0 at every width: the page cannot place two elements side by side, so a zero cannot distinguish a correct recreation from an untestable one.';
  } else if (!reflows) {
    grade = 'weak';
    why = 'page does not reflow between 400 and 1440 on both sides, so width-dependent divergence cannot be observed.';
  } else if (totalRec + totalSrc > 0) {
    grade = 'informative-positive';
    why = 'page is capable and reflows, and the instrument recorded divergence.';
  } else if (anyPairs) {
    grade = 'informative-negative';
    why = 'page is capable and reflows and the predicate did fire on this page, yet neither side diverged: a real clean result.';
  } else {
    grade = 'weak';
    why = 'capable by capacity but the predicate never found an overlapping pair on either side at any width, so the zero is instrument silence.';
  }

  out[site] = {
    recreationOnlyPerWidth: rec,
    sourceOnlyPerWidth: src,
    inlineRowCapacityPerWidth: cap,
    inlineRowCapacityMaxSource: capMax,
    widthDeltaPerWidth: delta,
    reflow: { source: reflowSrc, recreation: reflowRec },
    anyPairsEitherSide: anyPairs,
    totals: { recreationOnly: totalRec, sourceOnly: totalSrc },
    grade,
    gradeReason: why,
  };
}

const grades = Object.fromEntries(Object.entries(out).map(([k, v]) => [k, v.grade]));
const informative = Object.values(grades).filter((g) => g.startsWith('informative')).length;
writeFileSync(join(DIR, 'property-aggregate.json'), JSON.stringify({ widths: WIDTHS, sites: out, grades, informative, total: SITES.length }, null, 2));

for (const s of SITES) {
  const v = out[s];
  console.log(
    s.padEnd(10), v.grade.padEnd(22),
    'capMax=' + String(v.inlineRowCapacityMaxSource).padStart(4),
    'reflowSrc=' + String(v.reflow.source.movedPct).padStart(5),
    'reflowRec=' + String(v.reflow.recreation.movedPct).padStart(5),
    'pairs=' + (v.anyPairsEitherSide ? 'y' : 'n'),
    'rec=' + String(v.totals.recreationOnly).padStart(4),
    'src=' + String(v.totals.sourceOnly).padStart(4),
  );
}
const hdr = 'site'.padEnd(10) + WIDTHS.map((w) => String(w).padStart(7)).join('');
console.log('\nwidth-mismatch pct (same text, rec vs src):');
console.log(hdr);
for (const s of SITES) console.log(s.padEnd(10) + WIDTHS.map((w) => String(out[s].widthDeltaPerWidth[w].widthMismatchPct).padStart(7)).join(''));
console.log('\nmax width delta px:');
console.log(hdr);
for (const s of SITES) console.log(s.padEnd(10) + WIDTHS.map((w) => String(out[s].widthDeltaPerWidth[w].maxWidthDeltaPx).padStart(7)).join(''));
console.log('\ninformative ' + informative + ' / ' + SITES.length);
