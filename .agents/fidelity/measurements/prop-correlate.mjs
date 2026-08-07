// Per-site rank correlation between each candidate covariate and the defect count.
// Answers: does the covariate predict WHICH width fails, or only WHETHER a page fails?
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const A = JSON.parse(readFileSync(join(process.cwd(), 'property', 'property-aggregate.json'), 'utf8'));
const W = A.widths;

const rank = (v) => {
  const idx = v.map((x, i) => [x, i]).sort((a, b) => a[0] - b[0]);
  const r = new Array(v.length);
  let i = 0;
  while (i < idx.length) {
    let j = i;
    while (j + 1 < idx.length && idx[j + 1][0] === idx[i][0]) j++;
    const avg = (i + j) / 2 + 1;
    for (let k = i; k <= j; k++) r[idx[k][1]] = avg;
    i = j + 1;
  }
  return r;
};
const spearman = (a, b) => {
  if (new Set(a).size < 2 || new Set(b).size < 2) return null;
  const x = rank(a), y = rank(b), n = a.length;
  const mx = x.reduce((s, v) => s + v, 0) / n, my = y.reduce((s, v) => s + v, 0) / n;
  let num = 0, dx = 0, dy = 0;
  for (let i = 0; i < n; i++) { num += (x[i] - mx) * (y[i] - my); dx += (x[i] - mx) ** 2; dy += (y[i] - my) ** 2; }
  return +(num / Math.sqrt(dx * dy)).toFixed(3);
};

const RAW = Object.fromEntries(['godev', 'jquery', 'litenews', 'paper', 'phpman', 'plaintext'].map(
  (s) => [s, JSON.parse(readFileSync(join(process.cwd(), 'property', `raw-${s}.json`), 'utf8'))]));
const capOf = (site, w, k) => RAW[site].perWidth[w].inlineRowCapacity.source[k];

const rows = [];
for (const [site, v] of Object.entries(A.sites)) {
  if (v.grade === 'weak') continue;
  const defect = W.map((w) => v.recreationOnlyPerWidth[w]);
  const both = W.map((w) => v.recreationOnlyPerWidth[w] + v.sourceOnlyPerWidth[w]);
  rows.push({
    site,
    vs_widthMismatchPct: spearman(W.map((w) => v.widthDeltaPerWidth[w].widthMismatchPct), defect),
    vs_maxWidthDeltaPx: spearman(W.map((w) => v.widthDeltaPerWidth[w].maxWidthDeltaPx), defect),
    vs_inlineRowCapacity: spearman(W.map((w) => v.inlineRowCapacityPerWidth[w].source), defect),
    vs_inlineSiblingPairs: spearman(W.map((w) => capOf(site, w, 'inlineSiblingPairs')), defect),
    vs_navInlinePairs: spearman(W.map((w) => capOf(site, w, 'navInlinePairs')), defect),
    vs_viewportWidth: spearman(W.slice(), defect),
    bothDir_vs_maxWidthDeltaPx: spearman(W.map((w) => v.widthDeltaPerWidth[w].maxWidthDeltaPx), both),
  });
}
console.log('Spearman rho, recreation-only count vs covariate, across the 8 widths (informative pages only)\n');
console.table(rows);
