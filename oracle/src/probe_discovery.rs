pub const DISCOVER: &str = r#"(() => {
  const meaningful = e => {
    const r=e.getBoundingClientRect(), role=e.getAttribute('role') || '';
    const stateful=role==='tab'||e.hasAttribute('aria-pressed')||
      e.hasAttribute('aria-selected');
    const popup=e.hasAttribute('aria-haspopup')||e.hasAttribute('aria-expanded');
    const header=e.localName==='button'&&e.hasAttribute('aria-label')&&r.top<120;
    const entry=/^(input|select|textarea)$/.test(e.localName);
    return stateful||popup||header||entry;
  };
  const controls = [...document.querySelectorAll('button,input,select,textarea,[role="button"],[role="tab"]')]
    .filter(e => {
      const s = getComputedStyle(e), r = e.getBoundingClientRect();
      const active = e.getAttribute('aria-pressed') === 'true' ||
        e.getAttribute('aria-selected') === 'true';
      return meaningful(e) && !active && s.display !== 'none' && s.visibility !== 'hidden' &&
        r.width > 0 && r.height > 0;
    }).sort((a,b) => {
      const ar=a.getBoundingClientRect(), br=b.getBoundingClientRect();
      return ar.y-br.y || ar.x-br.x;
    });
  const occurrences = new Map();
  const rows = controls.map(e => {
    const key = [e.getAttribute('role') || '', e.getAttribute('aria-label') || '',
      e.children.length ? '' : (e.textContent || '').replace(/\s+/g,' ').trim(), e.localName].join('|');
    const occurrence = occurrences.get(key) || 0;
    occurrences.set(key, occurrence + 1);
    return {anchor:key+'@'+occurrence,key,persistent:e.getAttribute('role') === 'tab' ||
      e.hasAttribute('aria-pressed') || e.hasAttribute('aria-selected')};
  });
  const groups = new Map();
  for (const row of rows) {
    const group=groups.get(row.key)||[]; group.push(row); groups.set(row.key,group);
  }
  const selected=[...groups.values()].flatMap(group =>
    group.length<=2?group:[group[0],group[group.length-1]]);
  const anchors=selected.map(row=>row.anchor);
  const persistent=selected.map(row=>row.persistent);
  const source = document.documentElement.outerHTML;
  const opaque = [
    'eval(', 'WebAssembly', 'getContext("webgl', "getContext('webgl", 'new Worker(',
    'WebSocket(', 'EventSource(', 'XMLHttpRequest(', 'requestIdleCallback(',
    'Math.random(', 'Date.now(', 'new Date(', '<iframe', '<video', '<audio'
  ].filter(token => source.includes(token));
  const registrations = {
    timer: /setTimeout|setInterval/.test(source),
    animation: document.getAnimations().length > 0,
    media: [...document.styleSheets].some(sheet => {
      try { return [...sheet.cssRules].some(rule => rule.type === CSSRule.MEDIA_RULE); }
      catch { return true; }
    }),
    container: [...document.styleSheets].some(sheet => {
      try { return [...sheet.cssRules].some(rule => rule.cssText.startsWith('@container')); }
      catch { return true; }
    }),
    observer: /ResizeObserver|MutationObserver|IntersectionObserver/.test(source)
  };
  const boundaries = [];
  for (const sheet of document.styleSheets) {
    try {
      for (const rule of sheet.cssRules) {
        if (rule.conditionText) {
          for (const match of rule.conditionText.matchAll(/(\d+(?:\.\d+)?)px/g)) {
            const value = Number(match[1]);
            boundaries.push(Math.max(1,value-1),value,value+1);
          }
        }
      }
    } catch {}
  }
  for (const match of source.matchAll(/matchMedia\([^)]*?(\d+(?:\.\d+)?)px/g)) {
    const value=Number(match[1]); boundaries.push(Math.max(1,value-1),value,value+1);
  }
  const durations = document.getAnimations().map(a => Number(a.effect?.getTiming().duration))
    .filter(value => Number.isFinite(value) && value >= 0);
  const motionFrames = [...new Set(durations.flatMap(duration => {
    const frames=[]; for(let time=0;time<duration;time+=1000/60) frames.push(Math.round(time*1000)/1000);
    frames.push(duration); return frames;
  }))].sort((a,b)=>a-b);
  return {anchors, persistent, opaque, registrations,
    runtime: globalThis.__recreateOracle?.registrations || [],
    boundaries:[...new Set(boundaries)].sort((a,b)=>a-b), motionFrames,
    motionUnbounded: motionFrames.length > 600 || durations.some(value => value > 10000)};
})()"#;

pub const FIND_ANCHOR: &str = r#"anchor => {
  const meaningful = e => {
    const r=e.getBoundingClientRect(), role=e.getAttribute('role') || '';
    const stateful=role==='tab'||e.hasAttribute('aria-pressed')||
      e.hasAttribute('aria-selected');
    const popup=e.hasAttribute('aria-haspopup')||e.hasAttribute('aria-expanded');
    const header=e.localName==='button'&&e.hasAttribute('aria-label')&&r.top<120;
    const entry=/^(input|select|textarea)$/.test(e.localName);
    return stateful||popup||header||entry;
  };
  const controls = [...document.querySelectorAll('button,input,select,textarea,[role="button"],[role="tab"]')]
    .filter(e => {
      const s=getComputedStyle(e),r=e.getBoundingClientRect();
      return meaningful(e)&&s.display!=='none'&&s.visibility!=='hidden'&&
        r.width>0&&r.height>0;
    }).sort((a,b) => {
      const ar=a.getBoundingClientRect(),br=b.getBoundingClientRect();
      return ar.y-br.y||ar.x-br.x;
    });
  const occurrences=new Map();
  for (const e of controls) {
    const key=[e.getAttribute('role')||'',e.getAttribute('aria-label')||'',
      e.children.length?'':(e.textContent||'').replace(/\s+/g,' ').trim(),e.localName].join('|');
    const occurrence=occurrences.get(key)||0;
    occurrences.set(key,occurrence+1);
    if (key+'@'+occurrence===anchor) {
      const r=e.getBoundingClientRect();
      return {x:r.x+r.width/2,y:r.y+r.height/2};
    }
  }
  return null;
}"#;
