// Aggregates per-site survey records into one census, maps coverage against the
// fixture corpus by SCANNING each fixture's control markup (never by directory
// name), and emits the uncovered set sorted by descending site count.

import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const FIXTURES = join(HERE, '..', '..', '..', 'backtest', 'fixtures');

// --- feature definitions ----------------------------------------------------
// group: census section. detection: how the site side was measured.
// fixture: regex proving a fixture's control markup actually exercises it.
const FEATURES = [
  // layout systems
  ['flex', 'layout', 'computed', /display\s*:\s*(inline-)?flex/i],
  ['grid', 'layout', 'computed', /display\s*:\s*(inline-)?grid/i],
  ['float', 'layout', 'computed', /float\s*:\s*(left|right)/i],
  ['absolute', 'layout', 'computed', /position\s*:\s*absolute/i],
  ['fixed', 'layout', 'computed', /position\s*:\s*fixed/i],
  ['sticky', 'layout', 'computed', /position\s*:\s*sticky/i],
  ['container-query', 'layout', 'computed', /@container|container-type\s*:/i],
  ['subgrid', 'layout', 'computed', /subgrid/i],
  ['multi-column', 'layout', 'computed', /column-count|column-width|(^|[\s;{])columns\s*:/i],
  ['table-layout', 'layout', 'computed', /<table[\s>]|display\s*:\s*table/i],
  ['transform', 'layout', 'computed', /transform\s*:/i],
  ['aspect-ratio', 'layout', 'computed', /aspect-ratio\s*:/i],
  ['scroll-snap', 'layout', 'computed', /scroll-snap/i],
  ['css-custom-property', 'layout', 'computed', /--[a-z0-9-]+\s*:/i],
  ['z-index-stacking', 'layout', 'computed', /z-index\s*:/i],
  ['backdrop-filter', 'layout', 'computed', /backdrop-filter/i],
  ['clip-path', 'layout', 'computed', /clip-path/i],
  ['css-gradient', 'layout', 'computed', /gradient\(/i],
  ['box-shadow', 'layout', 'computed', /box-shadow\s*:/i],
  ['border-radius', 'layout', 'computed', /border-radius\s*:/i],
  ['css-transition', 'layout', 'computed', /transition\s*:|transition-duration/i],
  ['css-animation', 'layout', 'computed', /@keyframes|animation\s*:|animation-name/i],
  ['scroll-container', 'layout', 'computed', /overflow(-[xy])?\s*:\s*(auto|scroll)/i],
  ['webfont', 'layout', 'computed', /@font-face|fonts\.googleapis|\.woff/i],

  // interaction surfaces
  ['menu', 'interaction', 'aria/dom', /role\s*=\s*["']?menu|aria-haspopup|<menu[\s>]|popover/i],
  ['dialog', 'interaction', 'aria/dom', /<dialog[\s>]|role\s*=\s*["']?(dialog|alertdialog)|aria-modal/i],
  ['tooltip', 'interaction', 'aria/dom', /role\s*=\s*["']?tooltip|aria-describedby|data-tooltip/i],
  ['tab-set', 'interaction', 'aria/dom', /role\s*=\s*["']?tab(list|panel)?["'\s>]/i],
  ['accordion', 'interaction', 'aria/dom', /<details[\s>]|aria-controls/i],
  ['carousel', 'interaction', 'aria/dom', /carousel|aria-roledescription|scroll-snap-type\s*:\s*x/i],
  ['combobox', 'interaction', 'aria/dom', /role\s*=\s*["']?(combobox|listbox)|<select[\s>]|aria-autocomplete|list\s*=/i],
  ['drag-target', 'interaction', 'aria/dom', /draggable\s*=\s*["']?true|aria-grabbed|dnd-draggable/i],
  ['drag-listener-registrations', 'interaction', 'runtime (inflated by framework event delegation - see gaps)', /dragstart|dragover|ondrop/i],
  ['virtualized-list', 'interaction', 'aria/dom', /aria-setsize|virtual(ized|-scroll)/i],
  ['disclosure-button', 'interaction', 'aria/dom', /aria-expanded/i],
  ['live-region', 'interaction', 'aria/dom', /aria-live|role\s*=\s*["']?(status|alert|log)/i],
  ['form', 'interaction', 'dom', /<form[\s>]/i],
  ['custom-element', 'interaction', 'dom', /<[a-z]+-[a-z][a-z0-9-]*[\s>]|customElements/i],
  ['shadow-dom', 'interaction', 'dom+runtime', /attachShadow|<template\s+shadowroot/i],
  ['skip-link', 'interaction', 'dom', /href\s*=\s*["']#(main|content|main-content)|skip[- ]?to/i],
  ['iframe', 'interaction', 'dom', /<iframe[\s>]/i],
  ['video', 'interaction', 'dom', /<video[\s>]/i],
  ['canvas', 'interaction', 'dom', /<canvas[\s>]/i],
  ['svg', 'interaction', 'dom', /<svg[\s>]/i],
  ['responsive-image', 'interaction', 'dom', /srcset|<picture[\s>]/i],

  // responsive
  ['media-query', 'responsive', 'stylesheet-text', /@media/i],

  // state-carrying behaviors
  ['focus-trapping', 'behavior', 'proxy (modal surface present; trap only observable while open)', /inert\b|showModal|trapFocus|focus-trap/i],
  ['focus-restoration', 'behavior', 'runtime (wrapped HTMLElement.focus) + proxy', /\.focus\s*\(/i],
  ['scroll-locking', 'behavior', 'proxy (css lock rule / runtime overflow write)', /(html|body)[^{]*\{[^}]*overflow[^;}]*hidden|scroll-?lock|no-?scroll/i],
  ['optimistic-update', 'behavior', 'UNDETECTED - no DOM signature', /optimistic/i],
  ['deferred-load', 'behavior', 'rendered + runtime (IntersectionObserver wrap)', /loading\s*=\s*["']?lazy|IntersectionObserver|rel\s*=\s*["']?(preload|prefetch|modulepreload)|requestIdleCallback/i],
];

// Predicate deciding whether a site "uses" a feature, from its raw record.
function siteUses(rec, name) {
  if (!rec.ok) return null;
  const L = rec.layout || {}; const I = rec.interaction || {}; const B = rec.behavior || {};
  if (name === 'media-query') return (rec.breakpoints || []).length > 0;
  if (name in L) return L[name] > 0;
  if (name in I) return I[name] > 0;
  switch (name) {
    case 'focus-trapping': {
      const b = B['focus-trapping'] || {};
      return (b.modalSurfaces || 0) > 0 || (b.inert || 0) > 0 || (b.showModalCalls || 0) > 0;
    }
    case 'focus-restoration': {
      const b = B['focus-restoration'] || {};
      return (b.focusCallsAfterLoad || 0) > 0 || (b.focusCalls || 0) > 0 || (b.tabindexNegative || 0) > 0;
    }
    case 'scroll-locking': {
      const b = B['scroll-locking'] || {};
      return (b.cssLockRules || 0) > 0 || (b.runtimeOverflowWrites || 0) > 0 || b.bodyLockedAtRest === true;
    }
    case 'optimistic-update':
      return null; // instrument gap: not observable without mutating the site
    case 'deferred-load': {
      const b = B['deferred-load'] || {};
      return (b.lazyImgs || 0) > 0 || (b.ioObservedTargets || 0) > 0 || (b.preload || 0) > 0 || (b.idleCallbacks || 0) > 0;
    }
    default:
      return null;
  }
}

function siteEvidence(rec, name) {
  const L = rec.layout || {}; const I = rec.interaction || {}; const B = rec.behavior || {};
  if (name === 'media-query') return { breakpoints: (rec.breakpoints || []).length };
  if (name in L) return { matchedElements: L[name] };
  if (name in I) return { matchedElements: I[name] };
  return B[name] || {};
}

// --- fixture coverage: scan the control markup, not the directory name -------
function readFixtureCorpus() {
  const out = [];
  if (!existsSync(FIXTURES)) return out;
  for (const dir of readdirSync(FIXTURES)) {
    const full = join(FIXTURES, dir);
    if (!statSync(full).isDirectory()) continue;
    let text = '';
    const files = [];
    const walk = (p) => {
      for (const e of readdirSync(p)) {
        const f = join(p, e);
        const st = statSync(f);
        if (st.isDirectory()) { if (e !== 'mutations') walk(f); }
        else if (/\.(html|css|js|mjs)$/.test(e)) { files.push(f); text += '\n' + readFileSync(f, 'utf8'); }
      }
    };
    walk(full);
    out.push({ name: dir, files: files.length, text });
  }
  return out;
}

function main() {
  const raw = JSON.parse(readFileSync(join(HERE, 'raw', 'all-sites.json'), 'utf8'));
  const sites = raw.filter((r) => r.ok);
  const failed = raw.filter((r) => !r.ok);
  const corpus = readFixtureCorpus();

  // --- per-feature census ---
  const features = [];
  for (const [name, group, detection, fixtureRe] of FEATURES) {
    const using = [];
    let undetected = 0;
    for (const rec of sites) {
      const u = siteUses(rec, name);
      if (u === null) undetected++;
      else if (u) using.push(rec.id);
    }
    const fixtures = corpus.filter((f) => fixtureRe.test(f.text)).map((f) => f.name);
    features.push({
      feature: name, group, detection,
      siteCount: using.length,
      sites: using,
      undetectedOnSites: undetected,
      covered: fixtures.length > 0,
      coveringFixtures: fixtures,
    });
  }
  features.sort((a, b) => b.siteCount - a.siteCount || a.feature.localeCompare(b.feature));

  // --- breakpoint census ---
  const bpCount = new Map();
  for (const rec of sites) for (const w of rec.breakpoints || []) bpCount.set(w, (bpCount.get(w) || 0) + 1);
  const breakpoints = [...bpCount.entries()].map(([width, siteCount]) => ({ width, siteCount }))
    .sort((a, b) => b.siteCount - a.siteCount || a.width - b.width);

  // --- co-occurrence: real defects concentrate in feature interactions ---
  // "Identical site set" means the two features are perfectly coupled across the
  // sample, so a corpus that only isolates them cannot reproduce real usage.
  const MIN_PAIR_SITES = Math.ceil((sites.length * 2) / 3);
  const pairs = [];
  const common = features.filter((f) => f.siteCount >= MIN_PAIR_SITES);
  for (let i = 0; i < common.length; i++) {
    for (let j = i + 1; j < common.length; j++) {
      const a = common[i]; const b = common[j];
      const setB = new Set(b.sites);
      const both = a.sites.filter((s) => setB.has(s));
      const identical = both.length === a.siteCount && both.length === b.siteCount;
      // Only perfect coupling is reported: no surveyed site uses one without the
      // other, so an isolated fixture can never reproduce how they are really used.
      if (!identical) continue;
      const ra = FEATURES.find((x) => x[0] === a.feature)[3];
      const rb = FEATURES.find((x) => x[0] === b.feature)[3];
      const pairFixture = corpus.find((f) => ra.test(f.text) && rb.test(f.text));
      pairs.push({
        pair: [a.feature, b.feature],
        siteCount: both.length,
        coupling: identical ? 'identical-site-set' : 'high-co-occurrence',
        covered: !!pairFixture,
        coveringFixture: pairFixture ? pairFixture.name : null,
      });
    }
  }
  pairs.sort((a, b) => (a.coupling === b.coupling
    ? b.siteCount - a.siteCount
    : a.coupling === 'identical-site-set' ? -1 : 1));

  // --- instrument gaps ---
  const gaps = [];
  for (const [name, , detection] of FEATURES) {
    const f = features.find((x) => x.feature === name);
    if (/UNDETECTED/.test(detection)) gaps.push({ feature: name, reason: detection, undetectedSites: sites.length });
    else if (/proxy/.test(detection)) gaps.push({ feature: name, reason: `measured by proxy only: ${detection}`, proxySiteCount: f.siteCount });
  }
  const unreadable = sites.reduce((a, r) => a + (r.styleSheetsUnreadable || 0), 0);
  const readable = sites.reduce((a, r) => a + (r.styleSheetsRead || 0), 0);
  gaps.push({ feature: 'media-query', reason: 'stylesheet text retrieval', styleSheetsRead: readable, styleSheetsUnreadable: unreadable });
  gaps.push({ feature: 'drag-listener-registrations', reason: 'runtime listener count is inflated by framework root event delegation (React registers dragstart/dragover/drop on the container regardless of any drag UI); use drag-target for the DOM signal' });
  gaps.push({ feature: 'virtualized-list', reason: 'detected only via known library class names and aria-setsize overflow; a hand-rolled virtualizer with neither would read as absent' });
  for (const f of failed) gaps.push({ feature: `site:${f.id}`, reason: `survey failed: ${f.error}` });

  const census = {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    instrument: {
      engine: 'headless Chromium via Chrome DevTools Protocol',
      viewport: '1440x900',
      method: 'one pre-document instrumentation shim + one post-load census script per site; scroll pass to provoke deferred work; media queries read from raw stylesheet text via CSS.getStyleSheetText',
      principle: 'measured the computed/rendered result, not stylesheet source',
    },
    sitesSurveyed: sites.length,
    sitesAttempted: raw.length,
    frameworksCovered: [...new Set(sites.map((s) => s.framework))],
    featuresFound: features.filter((f) => f.siteCount > 0).length,
    featuresTracked: features.length,
    sitesDetail: raw.map((r) => ({
      id: r.id, url: r.url, framework: r.framework, why: r.why, ok: r.ok, error: r.error,
      elementCount: r.meta ? r.meta.elementCount : null,
      maxDepth: r.meta ? r.meta.maxDepth : null,
      styleSheetsRead: r.styleSheetsRead, styleSheetsUnreadable: r.styleSheetsUnreadable,
      breakpoints: r.breakpoints || [],
      frameworkSignals: r.meta ? r.meta.frameworkSignals : null,
      featuresUsed: FEATURES.map(([n]) => n).filter((n) => siteUses(r, n) === true),
      evidence: Object.fromEntries(FEATURES.map(([n]) => [n, siteEvidence(r, n)])),
    })),
    features,
    breakpoints,
    universalPairs: pairs,
    instrumentGaps: gaps,
    corpus: corpus.map((c) => ({ fixture: c.name, files: c.files })),
  };

  // --- uncovered set: >=2 sites, no fixture exercises it ---
  const uncovered = features
    .filter((f) => f.siteCount >= 2 && !f.covered)
    .map((f) => ({ feature: f.feature, group: f.group, siteCount: f.siteCount, sites: f.sites, detection: f.detection }))
    .sort((a, b) => b.siteCount - a.siteCount || a.feature.localeCompare(b.feature));

  const uncoveredPairs = pairs.filter((p) => !p.covered)
    .map((p) => ({ feature: `${p.pair[0]} + ${p.pair[1]}`, group: 'feature-pair', siteCount: p.siteCount, sites: [`${p.coupling}`], detection: 'co-occurrence across the surveyed sample' }))
    .sort((a, b) => b.siteCount - a.siteCount || a.feature.localeCompare(b.feature));

  const uncoveredDoc = {
    schemaVersion: 1,
    generatedAt: census.generatedAt,
    rule: 'feature used by >= 2 surveyed sites with no fixture whose control markup exercises it',
    sitesSurveyed: sites.length,
    uncoveredCount: uncovered.length,
    uncovered,
    uncoveredUniversalPairs: uncoveredPairs,
  };

  writeFileSync(join(HERE, 'census.json'), JSON.stringify(census, null, 2));
  writeFileSync(join(HERE, 'uncovered.json'), JSON.stringify(uncoveredDoc, null, 2));

  // --- markdown census ---
  const md = [];
  md.push('# Real-site feature census');
  md.push('');
  md.push(`Generated ${census.generatedAt} - ${census.sitesSurveyed}/${census.sitesAttempted} sites, ${census.frameworksCovered.length} frameworks.`);
  md.push('');
  md.push('Instrument: headless Chromium over CDP at 1440x900. One pre-document shim wraps IntersectionObserver, ResizeObserver, MutationObserver, HTMLElement.focus, dialog.showModal, attachShadow, addEventListener, history.pushState, fetch and overflow writes into counters. One post-load census script reads getComputedStyle and ARIA/DOM state. Media queries come from raw stylesheet text via CSS.getStyleSheetText, so cross-origin sheets are not silently dropped. Every number below is a measurement, not an estimate.');
  md.push('');
  md.push('## Sites surveyed');
  md.push('');
  md.push('| site | framework | why in scope | elements | depth | breakpoints | sheets read |');
  md.push('|---|---|---|---|---|---|---|');
  for (const s of census.sitesDetail) {
    md.push(`| ${s.id} | ${s.framework} | ${s.why} | ${s.ok ? s.elementCount : 'FAILED'} | ${s.ok ? s.maxDepth : '-'} | ${s.breakpoints.length} | ${s.ok ? `${s.styleSheetsRead}/${s.styleSheetsRead + s.styleSheetsUnreadable}` : '-'} |`);
  }
  md.push('');
  md.push('## Feature census');
  md.push('');
  md.push('| feature | group | sites | share | detection | fixture coverage |');
  md.push('|---|---|---|---|---|---|');
  for (const f of features) {
    const share = census.sitesSurveyed ? Math.round((f.siteCount / census.sitesSurveyed) * 100) : 0;
    md.push(`| ${f.feature} | ${f.group} | ${f.siteCount} | ${share}% | ${f.detection} | ${f.covered ? f.coveringFixtures.slice(0, 3).join(', ') : '**none**'} |`);
  }
  md.push('');
  md.push('## Declared media query breakpoint widths');
  md.push('');
  md.push('| width (px) | sites declaring |');
  md.push('|---|---|');
  for (const b of breakpoints) md.push(`| ${b.width} | ${b.siteCount} |`);
  md.push('');
  md.push('## Feature pairs that co-occur across the sample');
  md.push('');
  md.push('`identical-site-set` means the two features are used by exactly the same sites, so no site exercises one without the other.');
  md.push('');
  md.push('| pair | sites | coupling | single fixture exercising both |');
  md.push('|---|---|---|---|');
  for (const p of pairs) md.push(`| ${p.pair[0]} + ${p.pair[1]} | ${p.siteCount} | ${p.coupling} | ${p.covered ? p.coveringFixture : '**none**'} |`);
  md.push('');
  md.push('## Instrument gaps');
  md.push('');
  md.push('| item | reason | measurement |');
  md.push('|---|---|---|');
  for (const g of gaps) {
    const extra = Object.entries(g).filter(([k]) => k !== 'feature' && k !== 'reason').map(([k, v]) => `${k}=${v}`).join(' ');
    md.push(`| ${g.feature} | ${g.reason} | ${extra || '-'} |`);
  }
  writeFileSync(join(HERE, 'census.md'), md.join('\n') + '\n');

  const um = [];
  um.push('# Uncovered feature set');
  um.push('');
  um.push(`Rule: ${uncoveredDoc.rule}. ${uncoveredDoc.uncoveredCount} uncovered features across ${census.sitesSurveyed} surveyed sites.`);
  um.push('');
  um.push('| rank | feature | group | sites | sites using it |');
  um.push('|---|---|---|---|---|');
  uncovered.forEach((u, i) => um.push(`| ${i + 1} | ${u.feature} | ${u.group} | ${u.siteCount} | ${u.sites.join(', ')} |`));
  um.push('');
  um.push('## Uncovered feature pairs');
  um.push('');
  um.push('Pairs that co-occur across the sample with no single fixture exercising both. NIST fault data shows single-factor faults account for only 20-68% of failures while 2-way interactions reach 65-97%, so isolated fixtures leave these untested.');
  um.push('');
  um.push('| rank | pair | sites | coupling |');
  um.push('|---|---|---|---|');
  uncoveredPairs.forEach((u, i) => um.push(`| ${i + 1} | ${u.feature} | ${u.siteCount} | ${u.sites[0]} |`));
  writeFileSync(join(HERE, 'uncovered.md'), um.join('\n') + '\n');

  console.log(`sites surveyed:   ${census.sitesSurveyed}/${census.sitesAttempted}`);
  console.log(`frameworks:       ${census.frameworksCovered.length}`);
  console.log(`features found:   ${census.featuresFound} of ${census.featuresTracked} tracked`);
  console.log(`breakpoints:      ${breakpoints.length} distinct widths`);
  console.log(`uncovered:        ${uncovered.length} features, ${uncoveredPairs.length} uncovered universal pairs`);
  console.log('top 5 uncovered:');
  uncovered.slice(0, 5).forEach((u, i) => console.log(`  ${i + 1}. ${u.feature} - ${u.siteCount} sites`));
}

main();
