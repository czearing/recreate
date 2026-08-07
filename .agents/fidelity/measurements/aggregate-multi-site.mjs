import {readFileSync, writeFileSync} from 'node:fs';

const KEYS = ['cardgrid', 'dashboard', 'hero', 'form', 'legacy', 'table', 'docs'];
const META = {
  cardgrid: {structure: 'card grid', sourceUrl: 'https://getbootstrap.com/docs/5.3/examples/album/', stack: 'Bootstrap 5', why: 'Card grid: a wrapping flex/grid of equal cards, the shape the original complaint described.'},
  dashboard: {structure: 'dashboard with sidebar', sourceUrl: 'https://getbootstrap.com/docs/5.3/examples/dashboard/', stack: 'Bootstrap 5', why: 'Dashboard with a persistent sidebar: a fixed-position rail beside a fluid main column, so a frozen rail width can collide with content.'},
  hero: {structure: 'marketing page with hero', sourceUrl: 'https://vuejs.org/', stack: 'Vue 3 / VitePress', why: 'Marketing hero: large absolutely-positioned decorative art layered over headline text, the structure most able to turn a frozen length into overlap.'},
  form: {structure: 'form-heavy page', sourceUrl: 'https://getbootstrap.com/docs/5.3/examples/checkout/', stack: 'Bootstrap 5', why: 'Form-heavy: dense label/input/validation rows in a two-column grid that must collapse to one column.'},
  legacy: {structure: 'legacy table/float layout', sourceUrl: 'https://news.ycombinator.com/', stack: 'no framework, table layout', why: 'No-framework control built from nested tables and floats, the technique the research names as able to overlap when a parent collapses.'},
  table: {structure: 'data table', sourceUrl: 'https://datatables.net/examples/styling/bootstrap5.html', stack: 'jQuery / DataTables', why: 'Data table: a wide fixed-column grid that cannot reflow, plus a JS widget that rewrites the DOM after load.'},
  docs: {structure: 'documentation page', sourceUrl: 'https://docs.python.org/3/library/json.html', stack: 'Sphinx, no SPA framework', why: 'Documentation: long prose with a sidebar and many inline code spans; the highest element count in the sample.'},
};

const WIDTHS = [1440, 1076, 800, 661, 600, 480, 450, 400];
const sites = [];

for (const k of KEYS) {
  const raw = JSON.parse(readFileSync(`./multi-site/raw-${k}.json`, 'utf8'));
  const perWidth = {};
  const unionRecOnly = new Set();
  let maxRecOnly = 0, sumRecOnly = 0, recSideEverOverlapped = false, srcSideEverOverlapped = false;

  for (const w of WIDTHS) {
    const d = raw.perWidth[w];
    perWidth[w] = {
      sourcePairs: d.source.pairs,
      recreationPairs: d.recreation.pairs,
      recreationOnlyPairs: d.recreationOnly,
      sourceOnlyPairs: d.sourceOnly,
      recreationOnlyExamples: d.recreationOnlyExamples,
      sourceCandidates: d.source.candidates,
      recreationCandidates: d.recreation.candidates,
      sharedTextKeys: d.sharedTextKeys,
      sourceProtrudingElements: d.source.protruding,
      recreationProtrudingElements: d.recreation.protruding,
      recreationOnlyProtrusion: Math.max(0, d.recreation.protruding - d.source.protruding),
    };
    for (const p of d.recreationOnlyExamples) unionRecOnly.add(p);
    maxRecOnly = Math.max(maxRecOnly, d.recreationOnly);
    sumRecOnly += d.recreationOnly;
    if (d.recreation.pairs > 0) recSideEverOverlapped = true;
    if (d.source.pairs > 0) srcSideEverOverlapped = true;
  }

  // reflow control, per side, between the extreme widths
  const fp = (side, w) => new Map(raw.perWidth[w].boxes[side].map(b => [b[0], `${b[1]}/${b[2]}`]));
  const reflow = side => {
    const a = fp(side, 400), b = fp(side, 1440);
    let moved = 0, common = 0;
    for (const [t, v] of a) if (b.has(t)) { common++; if (b.get(t) !== v) moved++; }
    return {common, moved, pct: common ? +(100 * moved / common).toFixed(1) : null};
  };

  const widthsWithDefect = WIDTHS.filter(w => raw.perWidth[w].recreationOnly > 0);
  const protrusionWidths = WIDTHS.filter(w => perWidth[w].recreationOnlyProtrusion > 0);

  sites.push({
    key: k,
    ...META[k],
    recreationUrl: raw.site.recreationUrl,
    defect: {
      maxRecreationOnlyPairsAtAnyWidth: maxRecOnly,
      sumRecreationOnlyPairsAcrossWidths: sumRecOnly,
      widthsWithRecreationOnlyOverlap: widthsWithDefect,
      distinctRecreationOnlyPairsNamed: [...unionRecOnly].slice(0, 12),
      reproduces: maxRecOnly > 0,
    },
    secondarySignal: {
      note: 'Horizontal protrusion (scrollWidth > clientWidth with overflow-x visible). A frozen over-wide length becomes overflow, not overlap, in ordinary block/flex/grid flow, so overlap alone can read a real break as clean.',
      widthsWithRecreationOnlyProtrusion: protrusionWidths,
      maxRecreationOnlyProtrudingElements: Math.max(...WIDTHS.map(w => perWidth[w].recreationOnlyProtrusion)),
    },
    positiveControl: {
      bothSidesRendered: Math.min(...WIDTHS.map(w => perWidth[w].sourceCandidates)) > 0 && Math.min(...WIDTHS.map(w => perWidth[w].recreationCandidates)) > 0,
      minSourceCandidates: Math.min(...WIDTHS.map(w => perWidth[w].sourceCandidates)),
      minRecreationCandidates: Math.min(...WIDTHS.map(w => perWidth[w].recreationCandidates)),
      minSharedTextKeys: Math.min(...WIDTHS.map(w => perWidth[w].sharedTextKeys)),
      sourceReflow400to1440: reflow('source'),
      recreationReflow400to1440: reflow('recreation'),
      instrumentFoundOverlapOnSourceSide: srcSideEverOverlapped,
      instrumentFoundOverlapOnRecreationSide: recSideEverOverlapped,
      verdict: null,
    },
    exposure: {
      note: 'Raw counts rank by page size as much as by defect. Candidate elements is the opportunity each count was drawn from.',
      recreationCandidatesAt400: perWidth[400].recreationCandidates,
      recreationCandidatesAt1440: perWidth[1440].recreationCandidates,
    },
    perWidth,
  });
}

for (const s of sites) {
  const pc = s.positiveControl;
  pc.verdict = !pc.bothSidesRendered ? 'INVALID: a side did not render'
    : pc.recreationReflow400to1440.moved === 0 ? 'INVALID: recreation did not reflow'
    : pc.sourceReflow400to1440.moved === 0 ? 'INVALID: source did not reflow'
    : pc.instrumentFoundOverlapOnRecreationSide ? 'valid: instrument demonstrably sensitive on both sides of this page'
    : pc.instrumentFoundOverlapOnSourceSide ? 'valid with caveat: instrument found overlaps on this page (source side) but the recreation produced none at any width, so a recreation-side zero here is a real negative'
    : 'weak: this page produced no overlap on either side at any width, so its zero shows the page is not structurally able to overlap rather than that the recreation is correct';
}

// Second independent run of the three reproducing pages, fresh browser profile.
for (const s of sites) {
  let rep = null;
  try { rep = JSON.parse(readFileSync(`./multi-site/rep2-${s.key}.json`, 'utf8')); } catch {}
  s.stability = rep
    ? {
        repeats: 2,
        identicalAtEveryWidth: WIDTHS.every(w => rep.perWidth[w].recreationOnly === s.perWidth[w].recreationOnlyPairs),
        run2RecreationOnlyByWidth: Object.fromEntries(WIDTHS.map(w => [w, rep.perWidth[w].recreationOnly])),
      }
    : {repeats: 1, identicalAtEveryWidth: null, note: 'Repeated only for the reproducing pages; a zero page repeated would still be a zero.'};
}

sites.sort((a, b) => b.defect.maxRecreationOnlyPairsAtAnyWidth - a.defect.maxRecreationOnlyPairsAtAnyWidth
  || b.defect.sumRecreationOnlyPairsAcrossWidths - a.defect.sumRecreationOnlyPairsAcrossWidths);

const reproducing = sites.filter(s => s.defect.reproduces);

const out = {
  schemaVersion: 1,
  measurement: 'overlap-multi-site',
  commit: process.argv[2],
  measuredAt: new Date().toISOString(),
  question: 'Does the shipped generator produce element overlap that the source does not have, across structurally different real pages rather than one?',
  verdict: reproducing.length ? 'reproduces' : 'does-not-reproduce',
  verdictDetail: `${reproducing.length} of ${sites.length} pages show at least one pair that overlaps on the recreation and not on the source. ${sites.length - reproducing.length} pages show none. The defect is page-dependent, which is why a single-page measurement could return a clean zero and still leave the complaint open.`,
  priorMeasurement: {
    file: '.agents/fidelity/measurements/overlap-real-site.json',
    verdict: 'does-not-reproduce',
    reconciliation: 'That run measured one auth-gated page whose source side had to be replayed from a capture. It is not contradicted here: it remains a correct negative for that page. This run shows the negative did not generalise.',
  },
  method: {
    inheritedFrom: '.agents/fidelity/measurements/overlap-real-site.json',
    instrument: 'One headless Edge over CDP. Source and recreation opened as separate targets, Emulation.setDeviceMetricsOverride per width, identical predicate evaluated on each side.',
    overlapPredicate: 'Rect-rejection intersection between visible elements carrying their OWN non-empty text; ancestor/descendant pairs excluded; intersection area >= 16px2 and area/min(areaA,areaB) >= 0.08. Same thresholds as the single-site run.',
    pairIdentity: 'Each pair keyed by the sorted, whitespace-collapsed, lowercased visible text of its two elements, never by selector, because the recreation is a structural rewrite whose DOM paths legitimately differ. This also makes overlap-by-design cancel between the two sides.',
    differenceFromPriorRun: 'The source side is the LIVE page, not a replay, because every page here is public. That removes replay fidelity as a source of error entirely. In its place a content-drift guard scores a pair only when both of its texts are present on both sides, so content that changed between capture and measurement cannot manufacture a difference.',
    widths: WIDTHS,
    generation: 'recreate capture <url> --out <dir> (default viewports 1920x1080,1440x900,768x1024,390x844,320x568), then vite build, served from dist at the filesystem root so the emitted absolute /assets paths resolve.',
  },
  crossSiteFinding: {
    whereTheDefectConcentrates: 'Every named recreation-only pair on all three reproducing pages falls into one of two shapes: a horizontal row of short inline items (a breadcrumb trail, a top navigation bar), or a table cell against its table caption. Both are cases where elements share a horizontal line, which is the only situation in which a frozen over-wide length can become overlap rather than overflow.',
    whyFourPagesShowNothing: 'The four clean pages resolve their narrow layouts into single-column block flow. In block flow an over-wide box overflows its parent instead of colliding with a sibling, so those pages cannot express this defect as overlap however wrong their widths are. Their zeros are a property of the page structure, not a clearance of the generator.',
    correspondingOverflowEvidence: 'The card grid and dashboard pages, both of which score zero on overlap, show recreation-only horizontal protrusion at narrow widths (4 and 2 elements respectively at <=480px) that the source does not have. Measured on overlap alone they read as clean; they are not.',
    implicationForSampling: 'A one-page study of this defect is decided by whether that page happens to contain a horizontal inline row. Three of seven structurally distinct pages reproduce; picking any one of the other four would have returned a confident negative.',
  },
  instrumentGaps: [
    'Serving each recreation under a subpath returned a page that rendered zero elements, because the build emits absolute /assets/ URLs. That silent-zero was caught only by the per-site candidate-count control; without it the first run would have reported a clean zero for every site. Each site is now served at its own root.',
    'The capture command could not capture any MediaWiki page: en.wikipedia.org and simple.wikipedia.org both failed with "Space limit exceeded: Message too long" (78.3 MB and 119.4 MB against a 67.1 MB CDP cap). The data-table slot was filled with a DataTables page instead. This is a real capture-side limit on large pages, not a measurement choice.',
    'The headless browser exited on its own several times mid-run; runs were retried with a fresh profile until each produced a record. No partial record was used.',
    'Overlap is measured only between elements carrying their own text. A purely graphical overlap (an image over an image) is not observable to this instrument and is reported as undetected, not absent.',
  ],
  sites,
};

writeFileSync('./overlap-multi-site.json', JSON.stringify(out, null, 1));

console.log('rank\tsite       \tstructure                 \tmaxRecOnly\tsumRecOnly\twidths');
sites.forEach((s, i) => console.log(`${i + 1}\t${s.key.padEnd(11)}\t${s.structure.padEnd(26)}\t${String(s.defect.maxRecreationOnlyPairsAtAnyWidth).padStart(9)}\t${String(s.defect.sumRecreationOnlyPairsAcrossWidths).padStart(9)}\t${s.defect.widthsWithRecreationOnlyOverlap.join(',') || '-'}`));
console.log('\nverdict:', out.verdict, '|', reproducing.length, 'of', sites.length, 'reproduce');
for (const s of sites) console.log(s.key.padEnd(11), s.positiveControl.verdict);
