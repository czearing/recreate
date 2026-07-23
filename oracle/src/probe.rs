pub const INSTALL: &str = r#"(() => {
  if (globalThis.__recreateOracle) return;
  const pending = {timeouts: new Map(), intervals: new Map(), fetches: 0, xhrs: 0};
  const registrations = [], record = (kind, detail = '') =>
    registrations.push({kind, detail: String(detail)});
  let next = 0;
  const nativeTimeout = globalThis.setTimeout, nativeInterval = globalThis.setInterval;
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
  const nativeSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.send = function(...args) {
    record('xhr', this.responseURL);
    pending.xhrs++;
    this.addEventListener('loadend', () => pending.xhrs--, {once:true});
    return nativeSend.apply(this, args);
  };
  const nativeAdd = EventTarget.prototype.addEventListener;
  EventTarget.prototype.addEventListener = function(type, listener, options) {
    record('listener', type);
    return nativeAdd.call(this, type, listener, options);
  };
  const nativeRaf = globalThis.requestAnimationFrame;
  globalThis.requestAnimationFrame = fn => { record('raf'); return nativeRaf.call(globalThis, fn); };
  const nativeMicrotask = globalThis.queueMicrotask;
  globalThis.queueMicrotask = fn => {
    record('microtask'); return nativeMicrotask.call(globalThis, fn);
  };
  const nativeMedia = globalThis.matchMedia; globalThis.matchMedia = query => { record('media', query); return nativeMedia.call(globalThis, query); };
  const nativeAnimate = Element.prototype.animate;
  if (nativeAnimate) Element.prototype.animate = function(frames, options) {
    record('waapi', JSON.stringify(options)); return nativeAnimate.call(this, frames, options);
  };
  const nativeShadow = Element.prototype.attachShadow; Element.prototype.attachShadow = function(options) {
    record('shadow', options?.mode); return nativeShadow.call(this, options);
  };
  globalThis.__recreateOracle = {pending, registrations};
})()"#;

pub const SNAPSHOT: &str = r#"(() => {
  const rounded = n => Math.round(Number(n) * 1000) / 1000;
  const compact = value => {
    value=String(value); if (value.length<=256) return value;
    let hash=2166136261;
    for (let index=0;index<value.length;index++) hash=Math.imul(hash^value.charCodeAt(index),16777619);
    return `#${value.length}:${hash>>>0}`;
  };
  const styles = new WeakMap(), rects = new WeakMap(), paths = new WeakMap(), clipCache = new WeakMap();
  const style = e => { if (!styles.has(e)) styles.set(e,getComputedStyle(e)); return styles.get(e); };
  const styleNames=globalThis.__recreateOracleStyleNames||(()=>{
    const frame=document.body.appendChild(document.createElement('iframe'));
    frame.style.display='none';
    const probe=frame.contentDocument.body.appendChild(frame.contentDocument.createElement('div'));
    const names=[...frame.contentWindow.getComputedStyle(probe)];frame.remove();return names;
  })();
  globalThis.__recreateOracleStyleNames=styleNames;
  const rect = e => { if (!rects.has(e)) rects.set(e,e.getBoundingClientRect()); return rects.get(e); };
  const visible = e => {
    const s = style(e), r = rect(e);
    return s.display !== 'none' && s.visibility !== 'hidden' && r.width >= 0 && r.height >= 0;
  };
  const path = e => {
    if (paths.has(e)) return paths.get(e);
    let part=e.localName;
    if (e.id) part+='#'+e.id;
    else if (e.parentElement) {
      let index=1; for (let sibling=e.previousElementSibling;sibling;sibling=sibling.previousElementSibling) index++;
      part+=':nth-child('+index+')';
    }
    const value=e.parentElement?path(e.parentElement)+'>'+part:part;
    paths.set(e,value); return value;
  };
  const clips = e => {
    if (clipCache.has(e)) return clipCache.get(e);
    const p=e.parentElement, values=p?[...clips(p)]:[];
    if (p) {
      const s=style(p);
      if (/(hidden|clip|scroll|auto)/.test(s.overflow+s.overflowX+s.overflowY)) {
        const r=rect(p);
        values.push([rounded(r.x),rounded(r.y),rounded(r.width),rounded(r.height)]);
      }
    }
    clipCache.set(e,values); return values;
  };
  const semantic = e => {
    const s = style(e);
    const control = /^(a|button|input|select|textarea|img|svg|canvas|video|audio)$/.test(e.localName);
    const named = e.hasAttribute('role') || e.hasAttribute('aria-label') || e.hasAttribute('alt');
    const leafText = !e.children.length && (e.textContent || '').trim();
    const painted = s.backgroundColor !== 'rgba(0, 0, 0, 0)' ||
      ['borderTopWidth','borderRightWidth','borderBottomWidth','borderLeftWidth']
        .some(k => parseFloat(s[k]) > 0) || s.boxShadow !== 'none';
    return control || named || leafText || painted;
  };
  const actionState=globalThis.__recreateOracleAction;
  const active=document.activeElement;
  const scoped=actionState ? new Set() : null;
  if(scoped){
    const include=(e,tree=false)=>{
      if(!e||e.nodeType!==Node.ELEMENT_NODE)return;
      for(let n=e;n;n=n.parentElement)scoped.add(n);
      if(tree)for(const child of e.querySelectorAll('*'))scoped.add(child);
    };
    for(const node of actionState.dirtyNodes||[])include(node);
    for(const node of actionState.dirtyTrees||[])include(node,true);
    for(const animation of document.getAnimations())include(animation.effect?.target);
    include(active);
  }
  const candidates=scoped?[...scoped].filter(e=>e.isConnected):[...document.querySelectorAll('*')];
  const nodes = candidates.filter(e => visible(e) && semantic(e)).map(e => {
    const r = rect(e), s = style(e);
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
      style: Object.fromEntries(styleNames.map(k => [k, compact(s.getPropertyValue(k))])),
      pseudo: ['::before','::after'].map(p => {
        const ps=getComputedStyle(e,p);
        return {kind:p,content:compact(ps.content),display:ps.display,
          color:ps.color,background:compact(ps.background)};
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
  const locations = new Map();
  for (const node of nodes) {
    const key=node.anchor.split('@')[0]+'|'+node.rect.join(',');
    locations.set(key,(locations.get(key)||0)+1);
  }
  const ambiguous = nodes.filter(node =>
    locations.get(node.anchor.split('@')[0]+'|'+node.rect.join(','))>1).map(node => node.anchor);
  const animations = document.getAnimations().map(a => ({
    playState: a.playState, startTime: a.startTime, currentTime: a.currentTime,
    playbackRate: a.playbackRate, timing: a.effect?.getComputedTiming(),
    keyframes: a.effect?.getKeyframes()
  }));
  const action = globalThis.__recreateOracleActionResult || null;
  const pending = globalThis.__recreateOracle?.pending ||
    {timeouts:{size:0},intervals:{size:0},fetches:0,xhrs:0};
  const resources = performance.getEntriesByType('resource').map(entry => ({
    name: entry.name.startsWith('data:') ? entry.name : (() => {
      try { const u=new URL(entry.name); return u.pathname+u.search; } catch { return entry.name; }
    })(),
    initiator: entry.initiatorType
  }));
  return {
    url: location.href, title: document.title, nodes, animations,
    focus: active ? (nodes.find(node => node.path === path(active))?.anchor || '') : '',
    action, ambiguous,
    document: [document.documentElement.scrollWidth, document.documentElement.scrollHeight],
    visualViewport: visualViewport ? [visualViewport.width,visualViewport.height,visualViewport.scale,
      visualViewport.offsetLeft,visualViewport.offsetTop] : null,
    pending: {timeouts: pending.timeouts.size, intervals: pending.intervals.size,
      fetches: pending.fetches, xhrs: pending.xhrs},
    resources,
    documentState: Object.fromEntries(Object.entries(document.body?.dataset || {}).sort())
  };
})()"#;
