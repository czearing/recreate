// Real-site feature survey instrument.
// Drives a headless Chromium over CDP, injects one instrumentation shim before
// document creation and one census script after load, and records RENDERED
// feature usage plus every declared media-query breakpoint width.
//
// Usage: node survey.mjs

import { spawn } from 'node:child_process';
import { mkdtempSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = HERE;

const BROWSERS = [
  'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  'C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe',
];

export const SITES = [
  { id: 'nextjs', url: 'https://nextjs.org/', framework: 'Next.js (React)', why: 'Reference implementation of the dominant React meta-framework; App Router RSC output.' },
  { id: 'tailwindcss', url: 'https://tailwindcss.com/', framework: 'Next.js (React)', why: 'Utility-CSS site: densest source of declared breakpoints and container queries.' },
  { id: 'vercel', url: 'https://vercel.com/', framework: 'Next.js (React)', why: 'Commercial app shell by the framework authors; heavy motion and deferred load.' },
  { id: 'stripe', url: 'https://stripe.com/', framework: 'React', why: 'Industry benchmark for high-fidelity marketing layout and custom interaction surfaces.' },
  { id: 'github', url: 'https://github.com/', framework: 'React + Primer (Rails hybrid)', why: 'Large production app: menus, dialogs, tooltips, virtualization, partial hydration.' },
  { id: 'vuejs', url: 'https://vuejs.org/', framework: 'Vue 3 / VitePress', why: 'Vue ecosystem reference; SSG plus client hydration.' },
  { id: 'nuxt', url: 'https://nuxt.com/', framework: 'Nuxt 3 (Vue)', why: 'Vue meta-framework with island hydration and route-level deferred load.' },
  { id: 'angular', url: 'https://angular.dev/', framework: 'Angular', why: 'Angular reference site; CDK-driven overlay and a11y interaction surfaces.' },
  { id: 'svelte', url: 'https://svelte.dev/', framework: 'SvelteKit', why: 'Compiler-based framework with no virtual DOM; different runtime signature.' },
  { id: 'solidjs', url: 'https://docs.solidjs.com/', framework: 'SolidJS', why: 'Fine-grained reactivity framework; small-team CSS conventions.' },
  { id: 'astro', url: 'https://astro.build/', framework: 'Astro', why: 'Islands architecture: mostly static HTML with selectively hydrated widgets.' },
  { id: 'reactrouter', url: 'https://reactrouter.com/', framework: 'React Router / Remix', why: 'Data-router framework whose defining feature is optimistic UI and deferred data.' },
  { id: 'emberjs', url: 'https://emberjs.com/', framework: 'Ember', why: 'Older-generation MVC framework; legacy CSS idioms (float, absolute) survive here.' },
  { id: 'mdn', url: 'https://developer.mozilla.org/en-US/docs/Web/CSS/display', framework: 'Yari (React SSR + vanilla)', why: 'Long-form document page: multi-column, sticky sidebar, deep DOM.' },
  { id: 'wikipedia', url: 'https://en.wikipedia.org/wiki/Web_browser', framework: 'MediaWiki (vanilla JS)', why: 'No modern framework: float-based infoboxes, table layout, legacy CSS.' },
  { id: 'hackernews', url: 'https://news.ycombinator.com/', framework: 'None (static HTML)', why: 'Table-layout control case; proves the instrument reports absence, not noise.' },
  { id: 'bbc', url: 'https://www.bbc.com/news', framework: 'React (SSR)', why: 'High-traffic news grid: carousels, lazy media, many breakpoints.' },
  { id: 'shopify', url: 'https://www.shopify.com/', framework: 'Remix (React)', why: 'Large commerce marketing site; independent of the framework-docs monoculture.' },
];

// ---------------------------------------------------------------------------
// Injected shim: runs before any page script. Counts real runtime behavior.
// ---------------------------------------------------------------------------
const SHIM = `(() => {
  const M = { io: 0, ioObserved: 0, ro: 0, mo: 0, focusCalls: 0, focusAfterLoad: 0,
    showModal: 0, shadowRoots: 0, ric: 0,
    dragListeners: 0, keydownListeners: 0, pushState: 0, overflowWrites: 0,
    abortableFetch: 0, fetchCalls: 0, xhrCalls: 0 };
  window.__census = M;
  let loaded = false;
  addEventListener('load', () => { loaded = true; }, true);

  const IO = window.IntersectionObserver;
  if (IO) {
    const Wrapped = function (cb, opts) {
      M.io++;
      const inst = new IO(cb, opts);
      const obs = inst.observe.bind(inst);
      inst.observe = (el) => { M.ioObserved++; return obs(el); };
      return inst;
    };
    Wrapped.prototype = IO.prototype;
    window.IntersectionObserver = Wrapped;
  }
  const RO = window.ResizeObserver;
  if (RO) {
    const W = function (cb) { M.ro++; return new RO(cb); };
    W.prototype = RO.prototype; window.ResizeObserver = W;
  }
  const MO = window.MutationObserver;
  if (MO) {
    const W = function (cb) { M.mo++; return new MO(cb); };
    W.prototype = MO.prototype; window.MutationObserver = W;
  }

  const focus = HTMLElement.prototype.focus;
  HTMLElement.prototype.focus = function (...a) {
    M.focusCalls++; if (loaded) M.focusAfterLoad++; return focus.apply(this, a);
  };
  if (window.HTMLDialogElement) {
    const sm = HTMLDialogElement.prototype.showModal;
    HTMLDialogElement.prototype.showModal = function (...a) { M.showModal++; return sm.apply(this, a); };
  }
  const attach = Element.prototype.attachShadow;
  Element.prototype.attachShadow = function (...a) { M.shadowRoots++; return attach.apply(this, a); };
  const ric = window.requestIdleCallback;
  if (ric) window.requestIdleCallback = function (...a) { M.ric++; return ric.apply(window, a); };

  const ael = EventTarget.prototype.addEventListener;
  const DRAG = new Set(['dragstart','dragover','drop','dragend','dragenter','dragleave']);
  EventTarget.prototype.addEventListener = function (type, ...rest) {
    if (DRAG.has(type)) M.dragListeners++;
    if (type === 'keydown' || type === 'keyup') M.keydownListeners++;
    return ael.call(this, type, ...rest);
  };

  const ps = history.pushState;
  history.pushState = function (...a) { M.pushState++; return ps.apply(history, a); };

  const f = window.fetch;
  window.fetch = function (input, init) {
    M.fetchCalls++; if (init && init.signal) M.abortableFetch++;
    return f.call(window, input, init);
  };
  const xo = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function (...a) { M.xhrCalls++; return xo.apply(this, a); };

  const setProp = CSSStyleDeclaration.prototype.setProperty;
  CSSStyleDeclaration.prototype.setProperty = function (name, value, prio) {
    if (name === 'overflow' && (value === 'hidden' || value === 'clip')) M.overflowWrites++;
    return setProp.call(this, name, value, prio);
  };
  const desc = Object.getOwnPropertyDescriptor(CSSStyleDeclaration.prototype, 'overflow');
  if (desc && desc.set) {
    Object.defineProperty(CSSStyleDeclaration.prototype, 'overflow', {
      configurable: true, enumerable: desc.enumerable, get: desc.get,
      set: function (v) { if (v === 'hidden' || v === 'clip') M.overflowWrites++; return desc.set.call(this, v); },
    });
  }
})();`;

// ---------------------------------------------------------------------------
// Census script: one evaluate per site. Measures COMPUTED/rendered state.
// ---------------------------------------------------------------------------
const CENSUS = String.raw`(() => {
  const all = Array.from(document.querySelectorAll('*'));
  const cache = new Map();
  const style = (el) => { let v = cache.get(el); if (!v) { v = getComputedStyle(el); cache.set(el, v); } return v; };
  const q = (sel) => { try { return document.querySelectorAll(sel).length; } catch (e) { return 0; } };
  const count = (fn) => { let n = 0; for (const el of all) { try { if (fn(style(el), el)) n++; } catch (e) {} } return n; };

  const layout = {
    flex: count((s) => s.display === 'flex' || s.display === 'inline-flex'),
    grid: count((s) => s.display === 'grid' || s.display === 'inline-grid'),
    float: count((s) => s.float && s.float !== 'none'),
    absolute: count((s) => s.position === 'absolute'),
    fixed: count((s) => s.position === 'fixed'),
    sticky: count((s) => s.position === 'sticky'),
    'container-query': count((s) => s.containerType && s.containerType !== 'normal'),
    subgrid: count((s) => s.gridTemplateColumns === 'subgrid' || s.gridTemplateRows === 'subgrid'),
    'multi-column': count((s) => (s.columnCount && s.columnCount !== 'auto') || (s.columnWidth && s.columnWidth !== 'auto')),
    'table-layout': count((s) => s.display && s.display.indexOf('table') === 0),
    transform: count((s) => s.transform && s.transform !== 'none'),
    'aspect-ratio': count((s) => s.aspectRatio && s.aspectRatio !== 'auto'),
    'scroll-snap': count((s) => s.scrollSnapType && s.scrollSnapType !== 'none'),
    'css-custom-property': (() => { let n = 0; for (const el of all.slice(0, 1500)) { try { const s = style(el); for (let i = 0; i < s.length; i++) { if (s[i].charCodeAt(0) === 45 && s[i].charCodeAt(1) === 45) { n++; break; } } } catch (e) {} } return n; })(),
    'z-index-stacking': count((s) => s.zIndex && s.zIndex !== 'auto'),
    'backdrop-filter': count((s) => s.backdropFilter && s.backdropFilter !== 'none'),
    'clip-path': count((s) => s.clipPath && s.clipPath !== 'none'),
    'css-gradient': count((s) => /gradient\(/.test(s.backgroundImage || '')),
    'box-shadow': count((s) => s.boxShadow && s.boxShadow !== 'none'),
    'border-radius': count((s) => s.borderRadius && s.borderRadius !== '0px'),
    'css-transition': count((s) => s.transitionDuration && s.transitionDuration !== '0s'),
    'css-animation': count((s) => s.animationName && s.animationName !== 'none'),
    'scroll-container': count((s, el) => /(auto|scroll)/.test(s.overflowY + ' ' + s.overflowX) && el.scrollHeight > el.clientHeight + 8),
    'webfont': (() => { try { return document.fonts ? document.fonts.size : 0; } catch (e) { return 0; } })(),
  };

  const carouselLibs = q('.swiper,.slick-slider,.glide,.splide,.flickity-enabled,.embla,[data-carousel],[data-embla]');
  const carouselSnap = count((s, el) => s.scrollSnapType && s.scrollSnapType !== 'none' && el.scrollWidth > el.clientWidth + 8);
  const virtualLibs = q('[class*="react-window"],[class*="ReactVirtualized"],[class*="virtual-scroll"],cdk-virtual-scroll-viewport,[data-virtuoso-scroller],[class*="virtualized"],[class*="rc-virtual-list"]');
  const virtualAria = (() => { let n = 0; for (const el of all) {
      const ss = el.getAttribute && el.getAttribute('aria-setsize');
      if (ss && Number(ss) > 0 && el.parentElement && Number(ss) > el.parentElement.children.length) n++; } return n; })();
  const accordionAria = (() => { let n = 0; for (const el of all) {
      if (el.hasAttribute && el.hasAttribute('aria-expanded') && el.hasAttribute('aria-controls')) {
        const t = document.getElementById(el.getAttribute('aria-controls'));
        const r = t && (t.getAttribute('role') || '').toLowerCase();
        if (t && r !== 'menu' && r !== 'listbox' && r !== 'dialog') n++;
      } } return n; })();

  const interaction = {
    menu: q('[role=menu],[role=menubar],[role=menuitem],[role=menuitemcheckbox],[role=menuitemradio],[aria-haspopup=menu],[aria-haspopup=true],[popover]'),
    dialog: q('dialog,[role=dialog],[role=alertdialog],[aria-modal=true]'),
    tooltip: q('[role=tooltip],[data-tooltip],[data-tip],[aria-describedby]'),
    'tab-set': q('[role=tablist],[role=tab],[role=tabpanel]'),
    accordion: q('details,[data-accordion]') + accordionAria,
    carousel: q('[aria-roledescription=carousel],[aria-roledescription=slide]') + carouselLibs + carouselSnap,
    combobox: q('[role=combobox],[role=listbox],[aria-autocomplete],select,input[list]'),
    'drag-target': q('[draggable=true],[aria-grabbed],[data-rbd-draggable-id],[data-dnd-draggable],[data-dnd-kit-draggable],[class*="draggable"]'),
    'drag-listener-registrations': (window.__census ? window.__census.dragListeners : 0),
    'virtualized-list': virtualLibs + virtualAria,
    'disclosure-button': q('[aria-expanded]'),
    'live-region': q('[aria-live],[role=status],[role=alert],[role=log]'),
    form: q('form'),
    'custom-element': (() => { let n = 0; for (const el of all) { if (el.tagName.indexOf('-') > 0) n++; } return n; })(),
    'shadow-dom': (window.__census ? window.__census.shadowRoots : 0) + (() => { let n = 0; for (const el of all) { try { if (el.shadowRoot) n++; } catch (e) {} } return n; })(),
    'skip-link': q('a[href^="#"][class*=skip],a[href^="#"][class*=Skip],a[href="#main"],a[href="#content"],a[href="#main-content"]'),
    iframe: q('iframe'),
    video: q('video'),
    canvas: q('canvas'),
    svg: q('svg'),
    'responsive-image': q('picture,img[srcset],source[srcset]'),
  };

  const m = window.__census || {};
  const scrollLockRules = (() => {
    let n = 0;
    for (const sheet of Array.from(document.styleSheets)) {
      let rules; try { rules = sheet.cssRules; } catch (e) { continue; }
      if (!rules) continue;
      for (const r of Array.from(rules)) {
        const t = r.cssText || '';
        if (/(html|body)[^{]*\{[^}]*overflow[^:]*:\s*(hidden|clip)/.test(t)) n++;
      }
    }
    return n;
  })();
  const bodyStyle = getComputedStyle(document.body);

  const behavior = {
    'focus-trapping': {
      modalSurfaces: q('dialog,[role=dialog],[role=alertdialog],[aria-modal=true]'),
      inert: q('[inert]'),
      showModalCalls: m.showModal || 0,
      keyListeners: m.keydownListeners || 0,
      focusableInModal: q('[role=dialog] a,[role=dialog] button,dialog button'),
    },
    'focus-restoration': {
      focusCalls: m.focusCalls || 0,
      focusCallsAfterLoad: m.focusAfterLoad || 0,
      tabindexNegative: q('[tabindex="-1"]'),
      autofocus: q('[autofocus]'),
    },
    'scroll-locking': {
      cssLockRules: scrollLockRules,
      runtimeOverflowWrites: m.overflowWrites || 0,
      bodyLockedAtRest: bodyStyle.overflow === 'hidden' || bodyStyle.position === 'fixed',
      overscrollContained: count((s) => s.overscrollBehavior && s.overscrollBehavior !== 'auto'),
    },
    'optimistic-update': {
      pushState: m.pushState || 0,
      fetchCalls: m.fetchCalls || 0,
      abortableFetch: m.abortableFetch || 0,
      xhrCalls: m.xhrCalls || 0,
      forms: q('form'),
      clientRouterLinks: q('a[data-astro-prefetch],a[data-sveltekit-preload-data],[data-turbo],[data-remix],[data-prefetch]'),
      mutationObservers: m.mo || 0,
    },
    'deferred-load': {
      lazyImgs: q('img[loading=lazy],iframe[loading=lazy]'),
      asyncDecode: q('img[decoding=async]'),
      preload: q('link[rel=preload],link[rel=prefetch],link[rel=modulepreload]'),
      intersectionObservers: m.io || 0,
      ioObservedTargets: m.ioObserved || 0,
      idleCallbacks: m.ric || 0,
      resizeObservers: m.ro || 0,
      deferScripts: q('script[defer],script[async]'),
      moduleScripts: q('script[type=module]'),
    },
  };

  const meta = {
    url: location.href,
    title: document.title,
    elementCount: all.length,
    maxDepth: (() => { let d = 0; for (const el of all) { let n = 0, p = el; while (p) { n++; p = p.parentElement; } if (n > d) d = n; } return d; })(),
    styleSheetCount: document.styleSheets.length,
    crossOriginSheets: (() => { let n = 0; for (const s of Array.from(document.styleSheets)) { try { void s.cssRules; } catch (e) { n++; } } return n; })(),
    frameworkSignals: {
      react: !!(window.React || document.querySelector('[data-reactroot],#__next')) || all.slice(0, 400).some((e) => Object.keys(e).some((k) => k.indexOf('__react') === 0)),
      next: !!(window.__NEXT_DATA__ || document.getElementById('__next') || document.querySelector('script[src*="/_next/"]')),
      vue: !!(window.__VUE__ || document.querySelector('[data-v-app],[data-server-rendered]')) || all.slice(0, 400).some((e) => e.__vue_app__ || e.__vue__),
      nuxt: !!(window.__NUXT__ || document.getElementById('__nuxt')),
      angular: !!(window.ng || document.querySelector('[ng-version],app-root')),
      svelte: !!document.querySelector('[class*="svelte-"]') || !!window.__svelte,
      solid: !!document.querySelector('[data-hk]') || !!window._$HY,
      astro: !!document.querySelector('astro-island,[data-astro-cid],astro-slot,[data-astro-source-file]'),
      remix: !!(window.__remixContext || window.__reactRouterContext),
      ember: !!(window.Ember || document.querySelector('.ember-application,[id^=ember]')),
      htmx: !!window.htmx,
      jquery: !!window.jQuery,
    },
  };

  return { meta, layout, interaction, behavior };
})();`;

// ---------------------------------------------------------------------------
// Minimal CDP client
// ---------------------------------------------------------------------------
class CDP {
  constructor(ws) {
    this.ws = ws;
    this.id = 0;
    this.pending = new Map();
    this.handlers = new Map();
    ws.addEventListener('message', (ev) => {
      let msg;
      try { msg = JSON.parse(ev.data); } catch { return; }
      if (msg.id !== undefined) {
        const p = this.pending.get(msg.id);
        if (p) {
          this.pending.delete(msg.id);
          if (msg.error) p.reject(new Error(msg.error.message)); else p.resolve(msg.result);
        }
      } else {
        const hs = this.handlers.get(msg.method);
        if (hs) hs.forEach((h) => h(msg.params));
      }
    });
  }
  on(method, fn) {
    if (!this.handlers.has(method)) this.handlers.set(method, []);
    this.handlers.get(method).push(fn);
  }
  send(method, params = {}) {
    const id = ++this.id;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
      setTimeout(() => { if (this.pending.delete(id)) reject(new Error(`timeout ${method}`)); }, 45000);
    });
  }
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function connect(url) {
  const ws = new WebSocket(url);
  await new Promise((res, rej) => {
    ws.addEventListener('open', res, { once: true });
    ws.addEventListener('error', () => rej(new Error('ws error')), { once: true });
  });
  return new CDP(ws);
}

// ---------------------------------------------------------------------------
// Media query extraction from raw stylesheet text (cross-origin safe via CDP)
// ---------------------------------------------------------------------------
export function extractBreakpoints(cssText) {
  const widths = new Set();
  const conds = [];
  const re = /@(?:media|container)([^{]+)\{/g;
  let m;
  while ((m = re.exec(cssText)) !== null) conds.push(m[1].trim());
  const toPx = (num, unit) => {
    const n = Number(num);
    if (!Number.isFinite(n)) return null;
    if (unit === 'px') return Math.round(n);
    if (unit === 'em' || unit === 'rem') return Math.round(n * 16);
    if (unit === 'ch') return Math.round(n * 8);
    return null;
  };
  for (const c of conds) {
    let w;
    const wre = /(?:min|max)-(?:device-)?width\s*:\s*(-?[\d.]+)(px|em|rem|ch)/g;
    while ((w = wre.exec(c)) !== null) { const px = toPx(w[1], w[2]); if (px !== null && px > 0) widths.add(px); }
    const rre = /(?:^|[\s(])(?:width|inline-size)\s*[<>]=?\s*(-?[\d.]+)(px|em|rem|ch)/g;
    while ((w = rre.exec(c)) !== null) { const px = toPx(w[1], w[2]); if (px !== null && px > 0) widths.add(px); }
    const rre2 = /(-?[\d.]+)(px|em|rem|ch)\s*[<>]=?\s*(?:width|inline-size)/g;
    while ((w = rre2.exec(c)) !== null) { const px = toPx(w[1], w[2]); if (px !== null && px > 0) widths.add(px); }
  }
  return { widths: [...widths].sort((a, b) => a - b), conditions: conds };
}

async function surveySite(port, site) {
  const res = await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' });
  const target = await res.json();
  const cdp = await connect(target.webSocketDebuggerUrl);
  const sheetIds = [];
  const cssBodies = [];
  const record = { id: site.id, url: site.url, framework: site.framework, why: site.why, ok: false };
  try {
    cdp.on('CSS.styleSheetAdded', (p) => sheetIds.push(p.header.styleSheetId));
    await cdp.send('Page.enable');
    await cdp.send('Runtime.enable');
    await cdp.send('DOM.enable');
    await cdp.send('CSS.enable');
    await cdp.send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
    // A headless UA string trips bot walls and yields a challenge page instead of
    // the real site, which would silently poison the census.
    await cdp.send('Emulation.setUserAgentOverride', {
      userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
      acceptLanguage: 'en-US,en;q=0.9',
      platform: 'Win32',
    });
    await cdp.send('Page.addScriptToEvaluateOnNewDocument', { source: SHIM });

    const loaded = new Promise((r) => { cdp.on('Page.loadEventFired', r); setTimeout(r, 35000); });
    await cdp.send('Page.navigate', { url: site.url });
    await loaded;
    await sleep(6000);
    // provoke lazy work the way a user would: one scroll pass down and back
    await cdp.send('Runtime.evaluate', { expression: 'window.scrollTo(0, document.body.scrollHeight); "ok"' });
    await sleep(2500);
    await cdp.send('Runtime.evaluate', { expression: 'window.scrollTo(0, 0); "ok"' });
    await sleep(1500);

    const ev = await cdp.send('Runtime.evaluate', { expression: CENSUS, returnByValue: true, awaitPromise: true });
    if (ev.exceptionDetails) throw new Error(JSON.stringify(ev.exceptionDetails).slice(0, 600));
    Object.assign(record, ev.result.value, { ok: true });

    let failed = 0;
    for (const id of sheetIds) {
      try { const t = await cdp.send('CSS.getStyleSheetText', { styleSheetId: id }); cssBodies.push(t.text || ''); }
      catch { failed++; }
    }
    const bp = extractBreakpoints(cssBodies.join('\n'));
    record.breakpoints = bp.widths;
    record.mediaConditionCount = bp.conditions.length;
    record.styleSheetsRead = sheetIds.length - failed;
    record.styleSheetsUnreadable = failed;
    record.cssBytesRead = cssBodies.reduce((a, b) => a + b.length, 0);

    // Validity guard: a bot wall or error page must not enter the census as a
    // real measurement of the site it replaced.
    const title = (record.meta && record.meta.title) || '';
    const els = (record.meta && record.meta.elementCount) || 0;
    const wall = /just a moment|attention required|access denied|are you a robot|checking your browser|verify you are human|403 forbidden/i.test(title);
    if (wall || els < 60) {
      record.ok = false;
      record.error = `rejected: ${wall ? 'bot wall / challenge page' : 'implausibly small DOM'} (title="${title}", elements=${els})`;
    }
  } catch (err) {
    record.error = String((err && err.message) || err);
  } finally {
    try { cdp.ws.close(); } catch {}
    try { await fetch(`http://127.0.0.1:${port}/json/close/${target.id}`); } catch {}
  }
  return record;
}

async function main() {
  const browser = BROWSERS.find((p) => existsSync(p));
  if (!browser) throw new Error('no chromium found');
  const port = 9333 + Math.floor(Math.random() * 200);
  const profile = mkdtempSync(join(tmpdir(), 'census-'));
  const proc = spawn(browser, [
    '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`,
    '--no-first-run', '--no-default-browser-check', '--disable-extensions',
    '--disable-background-networking', '--disable-features=Translate,OptimizationHints',
    '--hide-scrollbars', '--window-size=1440,900', 'about:blank',
  ], { stdio: 'ignore' });

  let ready = false;
  for (let i = 0; i < 80 && !ready; i++) {
    try { const r = await fetch(`http://127.0.0.1:${port}/json/version`); if (r.ok) ready = true; } catch {}
    if (!ready) await sleep(500);
  }
  if (!ready) { proc.kill(); throw new Error('browser did not expose CDP'); }
  console.error(`[census] browser=${browser} port=${port}`);

  mkdirSync(join(OUT_DIR, 'raw'), { recursive: true });
  const records = [];
  for (const site of SITES) {
    const t0 = Date.now();
    let rec;
    try { rec = await surveySite(port, site); }
    catch (e) { rec = { id: site.id, url: site.url, framework: site.framework, why: site.why, ok: false, error: String(e.message || e) }; }
    rec.elapsedMs = Date.now() - t0;
    records.push(rec);
    console.error(`[census] ${site.id.padEnd(12)} ok=${rec.ok} bp=${(rec.breakpoints || []).length} el=${rec.meta ? rec.meta.elementCount : 0} css=${rec.styleSheetsRead || 0}/${(rec.styleSheetsRead || 0) + (rec.styleSheetsUnreadable || 0)} ${rec.error ? 'ERR ' + rec.error.slice(0, 120) : ''}`);
    writeFileSync(join(OUT_DIR, 'raw', `${site.id}.json`), JSON.stringify(rec, null, 2));
  }
  writeFileSync(join(OUT_DIR, 'raw', 'all-sites.json'), JSON.stringify(records, null, 2));
  proc.kill();
  console.error(`[census] done: ${records.filter((r) => r.ok).length}/${records.length} sites`);
}

if (process.argv[1] && process.argv[1].endsWith('survey.mjs')) {
  main().catch((e) => { console.error(e); process.exit(1); });
}
