pub const INSTALL: &str = r#"(() => {
  if (globalThis.__recreateOracle) return;
  const pending = {timeouts: new Map(), intervals: new Map(), fetches: 0};
  const registrations = [];
  const record = (kind, detail = '') => registrations.push({kind, detail: String(detail)});
  let next = 0;
  const nativeTimeout = globalThis.setTimeout;
  const nativeInterval = globalThis.setInterval;
  globalThis.setTimeout = (fn, delay = 0, ...args) => {
    record('timeout', delay);
    const id = ++next;
    const handle = nativeTimeout(() => {
      pending.timeouts.delete(id);
      fn(...args);
    }, delay);
    pending.timeouts.set(id, {delay: Number(delay), handle: String(handle)});
    return handle;
  };
  globalThis.setInterval = (fn, delay = 0, ...args) => {
    record('interval', delay);
    const id = ++next;
    const handle = nativeInterval(fn, delay, ...args);
    pending.intervals.set(id, {delay: Number(delay), handle: String(handle)});
    return handle;
  };
  const nativeFetch = globalThis.fetch;
  if (nativeFetch) globalThis.fetch = async (...args) => {
    record('fetch', args[0]);
    pending.fetches++;
    try { return await nativeFetch(...args); }
    finally { pending.fetches--; }
  };
  const nativeAdd = EventTarget.prototype.addEventListener;
  EventTarget.prototype.addEventListener = function(type, listener, options) {
    record('listener', type);
    return nativeAdd.call(this, type, listener, options);
  };
  const nativeRaf = globalThis.requestAnimationFrame;
  globalThis.requestAnimationFrame = fn => {
    record('raf');
    return nativeRaf.call(globalThis, fn);
  };
  const nativeMicrotask = globalThis.queueMicrotask;
  globalThis.queueMicrotask = fn => {
    record('microtask');
    return nativeMicrotask.call(globalThis, fn);
  };
  const nativeMedia = globalThis.matchMedia;
  globalThis.matchMedia = query => {
    record('media', query);
    return nativeMedia.call(globalThis, query);
  };
  const nativeAnimate = Element.prototype.animate;
  if (nativeAnimate) Element.prototype.animate = function(frames, options) {
    record('waapi', JSON.stringify(options));
    return nativeAnimate.call(this, frames, options);
  };
  const nativeShadow = Element.prototype.attachShadow;
  Element.prototype.attachShadow = function(options) {
    record('shadow', options?.mode);
    return nativeShadow.call(this, options);
  };
  globalThis.__recreateOracle = {pending, registrations};
})()"#;

pub const SNAPSHOT: &str = r#"(() => {
  const rounded = n => Math.round(Number(n) * 1000) / 1000;
  const visible = e => {
    const s = getComputedStyle(e);
    const r = e.getBoundingClientRect();
    return s.display !== 'none' && s.visibility !== 'hidden' && r.width >= 0 && r.height >= 0;
  };
  const path = e => {
    const parts = [];
    while (e && e.nodeType === 1) {
      let p = e.localName;
      if (e.id) p += '#' + e.id;
      else if (e.parentElement) p += ':nth-child(' + ([...e.parentElement.children].indexOf(e) + 1) + ')';
      parts.unshift(p);
      e = e.parentElement;
    }
    return parts.join('>');
  };
  const clips = e => {
    const values = [];
    for (let p=e.parentElement;p;p=p.parentElement) {
      const s=getComputedStyle(p);
      if (/(hidden|clip|scroll|auto)/.test(s.overflow+s.overflowX+s.overflowY)) {
        const r=p.getBoundingClientRect();
        values.push([rounded(r.x),rounded(r.y),rounded(r.width),rounded(r.height)]);
      }
    }
    return values;
  };
  const semantic = e => {
    const s = getComputedStyle(e);
    const control = /^(a|button|input|select|textarea|img|svg|canvas|video|audio)$/.test(e.localName);
    const named = e.hasAttribute('role') || e.hasAttribute('aria-label') || e.hasAttribute('alt');
    const leafText = !e.children.length && (e.textContent || '').trim();
    const painted = s.backgroundColor !== 'rgba(0, 0, 0, 0)' ||
      ['borderTopWidth','borderRightWidth','borderBottomWidth','borderLeftWidth']
        .some(k => parseFloat(s[k]) > 0) || s.boxShadow !== 'none';
    return control || named || leafText || painted;
  };
  const nodes = [...document.querySelectorAll('*')].filter(e => visible(e) && semantic(e)).map(e => {
    const r = e.getBoundingClientRect();
    const s = getComputedStyle(e);
    return {
      path: path(e), tag: e.localName, role: e.getAttribute('role') || '',
      name: e.getAttribute('aria-label') || e.getAttribute('alt') || '',
      text: e.children.length ? '' : (e.textContent || '').replace(/\s+/g, ' ').trim(),
      rect: [rounded(r.x),rounded(r.y),rounded(r.width),rounded(r.height)],
      boxes: {
        margin: [s.marginTop,s.marginRight,s.marginBottom,s.marginLeft],
        border: [s.borderTopWidth,s.borderRightWidth,s.borderBottomWidth,s.borderLeftWidth],
        padding: [s.paddingTop,s.paddingRight,s.paddingBottom,s.paddingLeft]
      },
      clips: clips(e),
      hit: document.elementFromPoint(r.x+r.width/2,r.y+r.height/2) === e,
      scroll: [e.scrollLeft,e.scrollTop,e.scrollWidth,e.scrollHeight],
      style: Object.fromEntries([...s].map(k => [k, s.getPropertyValue(k)])),
      pseudo: ['::before','::after'].map(p => {
        const ps=getComputedStyle(e,p);
        return {kind:p,content:ps.content,display:ps.display,color:ps.color,background:ps.background};
      }),
      state: {
        disabled: !!e.disabled, checked: !!e.checked, value: e.value ?? null,
        destination: e.href ? new URL(e.href,location.href).pathname +
          new URL(e.href,location.href).search + new URL(e.href,location.href).hash : null
      }
    };
  }).sort((a,b) => a.path.localeCompare(b.path));
  const occurrences = new Map();
  for (const node of nodes) {
    const key = [node.role,node.name,node.text,node.tag].join('|');
    const occurrence = occurrences.get(key) || 0;
    occurrences.set(key, occurrence + 1);
    node.anchor = key + '@' + occurrence;
  }
  const ambiguous = nodes.filter((node, index) => nodes.some((other, otherIndex) =>
    otherIndex !== index && node.anchor.split('@')[0] === other.anchor.split('@')[0] &&
    JSON.stringify(node.rect) === JSON.stringify(other.rect))).map(node => node.anchor);
  const animations = document.getAnimations().map(a => ({
    playState: a.playState, startTime: a.startTime, currentTime: a.currentTime,
    playbackRate: a.playbackRate, timing: a.effect?.getComputedTiming(),
    keyframes: a.effect?.getKeyframes()
  }));
  const active = document.activeElement;
  const pending = globalThis.__recreateOracle?.pending || {timeouts:{size:0},intervals:{size:0},fetches:0};
  const resources = performance.getEntriesByType('resource').map(entry => ({
    name: entry.name.startsWith('data:') ? entry.name : (() => {
      try { const u=new URL(entry.name); return u.pathname+u.search; } catch { return entry.name; }
    })(),
    initiator: entry.initiatorType
  }));
  return {
    url: location.href, title: document.title, nodes, animations,
    focus: active ? (nodes.find(node => node.path === path(active))?.anchor || '') : '',
    ambiguous,
    document: [document.documentElement.scrollWidth, document.documentElement.scrollHeight],
    visualViewport: visualViewport ? [visualViewport.width,visualViewport.height,visualViewport.scale,
      visualViewport.offsetLeft,visualViewport.offsetTop] : null,
    pending: {timeouts: pending.timeouts.size, intervals: pending.intervals.size, fetches: pending.fetches},
    resources,
    documentState: Object.fromEntries(Object.entries(document.body?.dataset || {}).sort())
  };
})()"#;
