// Property census: extends overlap-multi-site.mjs with a per-width covariate
// probe measuring INLINE-ROW CAPACITY (sibling pairs sharing a horizontal band).
//
// The overlap predicate below is copied VERBATIM from overlap-multi-site.mjs.
// Thresholds MIN_AREA=16 and MIN_FRAC=0.08 are unchanged, so the recreation-only
// and source-only counts here are directly comparable with overlap-multi-site.json.
// The covariate is a SECOND, independent expression; it never feeds the overlap
// numbers and does not alter them.
import {readFileSync, writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';

const CDP = process.argv[2] || 'http://127.0.0.1:9333';
const WIDTHS = [1440, 1076, 800, 661, 600, 480, 450, 400];

async function newTarget(url) {
  const r = await fetch(`${CDP}/json/new?${encodeURIComponent(url)}`, {method: 'PUT'});
  return r.json();
}
async function closeTarget(id) { await fetch(`${CDP}/json/close/${id}`); }

class Session {
  constructor(ws) { this.ws = ws; this.id = 0; this.pending = new Map(); }
  static async open(wsUrl) {
    const ws = new WebSocket(wsUrl);
    await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej; });
    const s = new Session(ws);
    ws.onmessage = e => {
      const m = JSON.parse(e.data);
      if (m.id && s.pending.has(m.id)) {
        const {res, rej} = s.pending.get(m.id);
        s.pending.delete(m.id);
        m.error ? rej(new Error(JSON.stringify(m.error))) : res(m.result);
      }
    };
    return s;
  }
  send(method, params = {}) {
    const id = ++this.id;
    return new Promise((res, rej) => {
      this.pending.set(id, {res, rej});
      this.ws.send(JSON.stringify({id, method, params}));
      setTimeout(() => { if (this.pending.has(id)) { this.pending.delete(id); rej(new Error(`timeout ${method}`)); } }, 90000);
    });
  }
  async eval(expr) {
    const r = await this.send('Runtime.evaluate', {
      expression: expr, returnByValue: true, awaitPromise: true,
    });
    if (r.exceptionDetails) throw new Error(r.exceptionDetails.text + ' ' + JSON.stringify(r.exceptionDetails.exception?.description || ''));
    return r.result.value;
  }
  close() { this.ws.close(); }
}

// ---- Overlap predicate loaded VERBATIM from overlap-multi-site.mjs at runtime.
// Extracting the bytes rather than copying them makes drift impossible: if the
// prior instrument ever changes, this run changes with it or fails loudly.
const PRIOR = readFileSync(new URL('./overlap-multi-site.mjs', import.meta.url), 'utf8');
const PROBE_START = PRIOR.indexOf('const PROBE = `');
const PROBE_END = PRIOR.indexOf('})()`;', PROBE_START);
if (PROBE_START < 0 || PROBE_END < 0) throw new Error('cannot locate PROBE in overlap-multi-site.mjs');
const PROBE_DECL = PRIOR.slice(PROBE_START, PROBE_END + 6);
const PROBE = PROBE_DECL.slice(PROBE_DECL.indexOf('`') + 1, PROBE_DECL.lastIndexOf('`'));
export const PROBE_SHA = createHash('sha256').update(PROBE).digest('hex').slice(0, 16);


// COVARIATE. Independent of the overlap predicate above.
// Inline-row capacity: among the element children of each parent, count sibling
// pairs whose vertical ranges intersect. That is the geometric definition of
// "sharing a horizontal line" and it detects a collapsed row (capacity -> 0)
// without reading class names or stylesheets, so it works across frameworks
// that implement collapse differently.
const COVARIATE = `(() => {
  const vis = el => {
    const cs = getComputedStyle(el);
    if (cs.display === 'none' || cs.visibility === 'hidden' || parseFloat(cs.opacity) === 0) return null;
    const r = el.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return null;
    return r;
  };
  let inlineSiblingPairs = 0;
  let rowParents = 0;
  let maxRowMembers = 0;
  let navInlinePairs = 0;
  const parents = [document.body, ...document.body.querySelectorAll('*')];
  for (const p of parents) {
    const kids = [];
    for (const c of p.children) { const r = vis(c); if (r) kids.push(r); }
    if (kids.length < 2) continue;
    let local = 0;
    for (let i = 0; i < kids.length; i++) for (let j = i + 1; j < kids.length; j++) {
      const a = kids[i], b = kids[j];
      const oy = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
      const minH = Math.min(a.height, b.height);
      // vertical ranges intersect by more than half the shorter box: same line
      if (oy > minH * 0.5) local++;
    }
    if (local > 0) {
      rowParents++;
      inlineSiblingPairs += local;
      const members = kids.length;
      if (members > maxRowMembers) maxRowMembers = members;
      const inNav = p.closest('nav,header,[role="navigation"],[class*="nav"],[class*="breadcrumb"],[class*="menu"],[id*="nav"]');
      if (inNav) navInlinePairs += local;
    }
  }
  return {inlineSiblingPairs, rowParents, maxRowMembers, navInlinePairs};
})()`;

async function measure(session, width) {
  await session.send('Emulation.setDeviceMetricsOverride', {
    width, height: 900, deviceScaleFactor: 1, mobile: false,
  });
  await new Promise(r => setTimeout(r, 700));
  const overlap = await session.eval(PROBE);
  const covariate = await session.eval(COVARIATE);
  return {...overlap, covariate};
}

async function openPage(url) {
  const t = await newTarget(url);
  const s = await Session.open(t.webSocketDebuggerUrl);
  await s.send('Page.enable');
  await s.send('Runtime.enable');
  await s.send('Emulation.setDeviceMetricsOverride', {width: 1440, height: 900, deviceScaleFactor: 1, mobile: false});
  await new Promise(r => setTimeout(r, 3500));
  await s.eval(`(async()=>{window.scrollTo(0,document.body.scrollHeight);await new Promise(r=>setTimeout(r,900));window.scrollTo(0,0);await new Promise(r=>setTimeout(r,600));return 1})()`);
  return {targetId: t.id, session: s};
}

export async function runSite(site) {
  const src = await openPage(site.sourceUrl);
  const rec = await openPage(site.recreationUrl);
  const perWidth = {};
  try {
    for (const w of WIDTHS) {
      const a = await measure(src.session, w);
      const b = await measure(rec.session, w);

      const aSet = new Set(a.textKeys), bSet = new Set(b.textKeys);
      const shared = k => k.split(' ||| ').every(t => aSet.has(t) && bSet.has(t));
      const aPairs = new Set(a.pairs), bPairs = new Set(b.pairs);
      const recOnly = [...bPairs].filter(k => !aPairs.has(k) && shared(k));
      const srcOnly = [...aPairs].filter(k => !bPairs.has(k) && shared(k));

      perWidth[w] = {
        source: {pairs: aPairs.size, candidates: a.candidates, protruding: a.protrudingElements, scrollWidth: a.docScrollWidth, clientWidth: a.clientWidth},
        recreation: {pairs: bPairs.size, candidates: b.candidates, protruding: b.protrudingElements, scrollWidth: b.docScrollWidth, clientWidth: b.clientWidth},
        recreationOnly: recOnly.length,
        sourceOnly: srcOnly.length,
        recreationOnlyExamples: recOnly.slice(0, 8),
        sourceOnlyExamples: srcOnly.slice(0, 8),
        sharedTextKeys: [...aSet].filter(t => bSet.has(t)).length,
        inlineRowCapacity: {source: a.covariate, recreation: b.covariate},
        boxes: {source: a.boxes, recreation: b.boxes},
      };
    }
  } finally {
    src.session.close(); rec.session.close();
    await closeTarget(src.targetId); await closeTarget(rec.targetId);
  }
  return perWidth;
}

if (process.argv[3]) {
  const site = JSON.parse(readFileSync(process.argv[3], 'utf8'));
  const perWidth = await runSite(site);
  writeFileSync(process.argv[4], JSON.stringify({site, perWidth}, null, 1));
  for (const [w, d] of Object.entries(perWidth)) {
    console.log(`${w}\tsrc=${d.source.pairs}\trec=${d.recreation.pairs}\trecOnly=${d.recreationOnly}\tsrcOnly=${d.sourceOnly}\trowSrc=${d.inlineRowCapacity.source.inlineSiblingPairs}\trowRec=${d.inlineRowCapacity.recreation.inlineSiblingPairs}`);
  }
}

