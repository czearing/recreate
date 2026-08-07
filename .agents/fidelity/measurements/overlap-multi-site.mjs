// Multi-site narrow-width overlap differential.
// Same both-sides method as .agents/fidelity/measurements/overlap-real-site.json:
// one browser, both pages, identical predicate, pairs keyed by sorted visible text.
import {readFileSync, writeFileSync} from 'node:fs';

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

// The page-side predicate. Identical for source and recreation.
// Thresholds match the ones the comparator itself uses, per the single-site run.
const PROBE = `(() => {
  const MIN_AREA = 16, MIN_FRAC = 0.08;
  const norm = t => (t || '').replace(/\\s+/g, ' ').trim().toLowerCase();
  const els = [];
  for (const el of document.body.querySelectorAll('*')) {
    const cs = getComputedStyle(el);
    if (cs.display === 'none' || cs.visibility === 'hidden' || parseFloat(cs.opacity) === 0) continue;
    const r = el.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) continue;
    // own visible text only: a wrapper inherits its children's text and would
    // otherwise be reported as overlapping everything it contains
    let own = '';
    for (const n of el.childNodes) if (n.nodeType === 3) own += n.textContent;
    const t = norm(own);
    if (!t) continue;
    els.push({el, t, r});
  }
  const isAnc = (a, b) => a.contains(b) || b.contains(a);
  const pairs = [];
  for (let i = 0; i < els.length; i++) for (let j = i + 1; j < els.length; j++) {
    const A = els[i], B = els[j];
    if (isAnc(A.el, B.el)) continue;
    const ox = Math.min(A.r.right, B.r.right) - Math.max(A.r.left, B.r.left);
    const oy = Math.min(A.r.bottom, B.r.bottom) - Math.max(A.r.top, B.r.top);
    if (ox <= 0 || oy <= 0) continue;
    const area = ox * oy;
    if (area < MIN_AREA) continue;
    const aA = A.r.width * A.r.height, aB = B.r.width * B.r.height;
    if (area / Math.min(aA, aB) < MIN_FRAC) continue;
    pairs.push([A.t, B.t].sort().join(' ||| '));
  }
  // horizontal overflow: the other way a frozen length breaks a page
  const de = document.documentElement;
  let protruding = 0;
  for (const el of document.body.querySelectorAll('*')) {
    if (el.scrollWidth > el.clientWidth + 1 && getComputedStyle(el).overflowX === 'visible') protruding++;
  }
  return {
    pairs,
    textKeys: els.map(e => e.t),
    candidates: els.length,
    docScrollWidth: de.scrollWidth,
    clientWidth: de.clientWidth,
    protrudingElements: protruding,
    bodyTextLen: norm(document.body.innerText).length,
    // geometry fingerprint for the reflow check
    boxes: els.slice(0, 400).map(e => [e.t, Math.round(e.r.left), Math.round(e.r.width)]),
  };
})()`;

async function measure(session, width) {
  await session.send('Emulation.setDeviceMetricsOverride', {
    width, height: 900, deviceScaleFactor: 1, mobile: false,
  });
  await new Promise(r => setTimeout(r, 700));
  return session.eval(PROBE);
}

async function openPage(url) {
  const t = await newTarget(url);
  const s = await Session.open(t.webSocketDebuggerUrl);
  await s.send('Page.enable');
  await s.send('Runtime.enable');
  await s.send('Emulation.setDeviceMetricsOverride', {width: 1440, height: 900, deviceScaleFactor: 1, mobile: false});
  // settle: wait for load then provoke lazy content
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

      // Drift guard: only score a pair when BOTH of its texts exist on BOTH
      // sides. A live page that changed between capture and measurement would
      // otherwise manufacture a difference that is content, not layout.
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
        recreationOnlyExamples: recOnly.slice(0, 6),
        sourceOnlyExamples: srcOnly.slice(0, 6),
        sharedTextKeys: [...aSet].filter(t => bSet.has(t)).length,
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
    console.log(`${w}\tsrc=${d.source.pairs}\trec=${d.recreation.pairs}\trecOnly=${d.recreationOnly}\tsrcOnly=${d.sourceOnly}\tovfSrc=${d.source.protruding}\tovfRec=${d.recreation.protruding}`);
  }
}
