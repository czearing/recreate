// Mechanism signature test: frozen derived geometry must be most-nearly-exact
// at a SAMPLED capture width. Sampled widths are 1920,1440,768,390,320
// (src/cli.rs:77). Of the probe sweep 400,450,480,600,661,800,1076,1440 only
// 1440 is sampled. Within-page test: needs no clean pages, so class balance
// cannot confound it. Tolerance 2px, unchanged from the locked run.
import fs from 'node:fs';

const TOL = 2;
const SAMPLED = 1440;
const WIDTHS = [400, 450, 480, 600, 661, 800, 1076, 1440];
const SITES = ['reactdev', 'sveltedev', 'vuejs', 'cern', 'danluu', 'nprtext', 'lobsters', 'gnu', 'w3c', 'sourcehut'];
const BATCH1 = new Set(['reactdev', 'sveltedev', 'vuejs', 'cern', 'danluu', 'nprtext']);

function widthStats(rec, w) {
  const p = rec.perWidth[String(w)];
  if (!p || !p.boxes) return null;
  const src = new Map(), rc = new Map();
  for (const [t, l, wd] of p.boxes.source || []) if (!src.has(t)) src.set(t, wd);
  for (const [t, l, wd] of p.boxes.recreation || []) if (!rc.has(t)) rc.set(t, wd);
  let shared = 0, bad = 0, sumAbs = 0, maxAbs = 0, recWider = 0, srcWider = 0;
  for (const [t, sw] of src) {
    if (!rc.has(t)) continue;
    const d = rc.get(t) - sw;
    shared++; sumAbs += Math.abs(d);
    if (Math.abs(d) > maxAbs) maxAbs = Math.abs(d);
    if (Math.abs(d) > TOL) { bad++; d > 0 ? recWider++ : srcWider++; }
  }
  if (!shared) return null;
  return {
    sharedTexts: shared, mismatched: bad,
    pct: +(100 * bad / shared).toFixed(1),
    meanAbsDeltaPx: +(sumAbs / shared).toFixed(2),
    maxAbsDeltaPx: Math.round(maxAbs),
    recreationWider: recWider, sourceWider: srcWider,
  };
}

const out = {};
for (const s of SITES) {
  const f = `wf/raw-${s}.json`;
  if (!fs.existsSync(f)) continue;
  const rec = JSON.parse(fs.readFileSync(f, 'utf8'));
  const per = {};
  for (const w of WIDTHS) per[w] = widthStats(rec, w);
  const usable = WIDTHS.filter(w => per[w]);
  const batch = BATCH1.has(s) ? 1 : 2;
  if (!usable.length) { out[s] = { batch, perWidth: per, signature: 'untestable-no-shared-texts' }; continue; }

  const byPct = [...usable].sort((a, b) => per[a].pct - per[b].pct || per[a].meanAbsDeltaPx - per[b].meanAbsDeltaPx);
  const byMean = [...usable].sort((a, b) => per[a].meanAbsDeltaPx - per[b].meanAbsDeltaPx);
  const uns = usable.filter(w => w !== SAMPLED);
  const mean = (arr, k) => arr.length ? +(arr.reduce((a, w) => a + per[w][k], 0) / arr.length).toFixed(2) : null;

  out[s] = {
    batch, perWidth: per,
    sampledWidth: SAMPLED,
    sampledIsArgminByPct: byPct[0] === SAMPLED,
    sampledIsArgminByMeanDelta: byMean[0] === SAMPLED,
    rankOfSampledByPct: byPct.indexOf(SAMPLED) + 1,
    widthsCompared: usable.length,
    pctAtSampled: per[SAMPLED] ? per[SAMPLED].pct : null,
    meanPctAtUnsampled: mean(uns, 'pct'),
    meanDeltaAtSampled: per[SAMPLED] ? per[SAMPLED].meanAbsDeltaPx : null,
    meanDeltaAtUnsampled: mean(uns, 'meanAbsDeltaPx'),
  };
}
fs.mkdirSync('wf', { recursive: true });
fs.writeFileSync('wf/wf-signature.json', JSON.stringify(out, null, 2));

const hdr = 'site        b' + WIDTHS.map(w => String(w).padStart(7)).join('');
console.log('== width-mismatch %% (>2px) ==\n' + hdr);
for (const [s, v] of Object.entries(out)) {
  const row = WIDTHS.map(w => (v.perWidth[w] ? String(v.perWidth[w].pct) : '-').padStart(7)).join('');
  console.log(s.padEnd(11) + String(v.batch).padEnd(2) + row +
    '   argmin@1440=' + (v.sampledIsArgminByPct ?? '-') + ' rank=' + (v.rankOfSampledByPct ?? '-') + '/' + (v.widthsCompared ?? 0));
}
console.log('\n== mean |width delta| px ==\n' + hdr);
for (const [s, v] of Object.entries(out)) {
  const row = WIDTHS.map(w => (v.perWidth[w] ? String(v.perWidth[w].meanAbsDeltaPx) : '-').padStart(7)).join('');
  console.log(s.padEnd(11) + String(v.batch).padEnd(2) + row + '   argminDelta@1440=' + (v.sampledIsArgminByMeanDelta ?? '-'));
}
console.log('\n== shared texts (exposure) ==\n' + hdr);
for (const [s, v] of Object.entries(out)) {
  console.log(s.padEnd(11) + String(v.batch).padEnd(2) +
    WIDTHS.map(w => (v.perWidth[w] ? String(v.perWidth[w].sharedTexts) : '-').padStart(7)).join(''));
}
