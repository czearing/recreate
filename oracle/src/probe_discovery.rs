pub const DISCOVER: &str = r#"(diagnostic => {
  const selector='a[href],button,input:not([type="hidden"]),select,textarea,summary,'+
    '[role="button"],[role="tab"],[role="menuitem"],[role="option"],[role="checkbox"],'+
    '[role="radio"],[role="switch"],[role="slider"],[contenteditable="true"],'+
    '[tabindex]:not([tabindex="-1"])';
  const controls = [...document.querySelectorAll(selector)]
    .filter(e => {
      const s = getComputedStyle(e), r = e.getBoundingClientRect();
      return !e.disabled&&e.getAttribute('aria-disabled')!=='true'&&
        s.display!=='none'&&s.visibility!=='hidden'&&r.width>0&&r.height>0;
    }).sort((a,b) => {
      const ar=a.getBoundingClientRect(), br=b.getBoundingClientRect();
      return ar.y-br.y || ar.x-br.x;
    });
  const occurrences = new Map();
  const rows = controls.map(e => {
    const key = [e.getAttribute('role') || '', e.getAttribute('aria-label') || '',
      '', e.localName].join('|');
    const occurrence = occurrences.get(key) || 0;
    occurrences.set(key, occurrence + 1);
    return {anchor:key+'@'+occurrence};
  });
  const anchors=rows.map(row=>row.anchor);
  const opaque = [...document.querySelectorAll('iframe,video,audio,canvas')]
    .map(element => element.localName);
  const runtime = globalThis.__recreateOracle?.registrations || [];
  const registrations = {
    timer: runtime.some(item => item.kind === 'timeout' || item.kind === 'interval'),
    animation: document.getAnimations().length > 0,
    media: false,
    container: false,
    observer: false
  };
  const boundaries = [];
  const durations = diagnostic ? [] : document.getAnimations()
    .map(a => Number(a.effect?.getTiming().duration))
    .filter(value => Number.isFinite(value) && value >= 0);
  const motionFrames = [...new Set(durations.flatMap(duration => {
    const frames=[]; for(let time=0;time<duration;time+=1000/60) frames.push(Math.round(time*1000)/1000);
    frames.push(duration); return frames;
  }))].sort((a,b)=>a-b);
  return {anchors, controls:rows, opaque, registrations,
    runtime,
    boundaries:[...new Set(boundaries)].sort((a,b)=>a-b), motionFrames,
    motionUnbounded: motionFrames.length > 600 || durations.some(value => value > 10000)};
})"#;

pub const FIND_ANCHOR: &str = r#"anchor => {
  const selector='a[href],button,input:not([type="hidden"]),select,textarea,summary,'+
    '[role="button"],[role="tab"],[role="menuitem"],[role="option"],[role="checkbox"],'+
    '[role="radio"],[role="switch"],[role="slider"],[contenteditable="true"],'+
    '[tabindex]:not([tabindex="-1"])';
  const controls = [...document.querySelectorAll(selector)]
    .filter(e => {
      const s=getComputedStyle(e),r=e.getBoundingClientRect();
      return !e.disabled&&e.getAttribute('aria-disabled')!=='true'&&
        s.display!=='none'&&s.visibility!=='hidden'&&r.width>0&&r.height>0;
    }).sort((a,b) => {
      const ar=a.getBoundingClientRect(),br=b.getBoundingClientRect();
      return ar.y-br.y||ar.x-br.x;
    });
  const occurrences=new Map();
  const [wantedKey,wantedOccurrenceText]=anchor.split('@');
  const [wantedRole,wantedLabel,,wantedTag]=wantedKey.split('|');
  const wantedOccurrence=Number(wantedOccurrenceText);
  let fallbackOccurrence=0,fallback=null;
  for (const e of controls) {
    const key=[e.getAttribute('role')||'',e.getAttribute('aria-label')||'',
      '',e.localName].join('|');
    const occurrence=occurrences.get(key)||0;
    occurrences.set(key,occurrence+1);
    if (key+'@'+occurrence===anchor) {
      const r=e.getBoundingClientRect();
      return {x:r.x+r.width/2,y:r.y+r.height/2};
    }
    if((e.getAttribute('role')||'')===wantedRole&&
      (e.getAttribute('aria-label')||'')===wantedLabel&&e.localName===wantedTag){
      if(fallbackOccurrence===wantedOccurrence){
        const r=e.getBoundingClientRect();
        fallback={x:r.x+r.width/2,y:r.y+r.height/2};
      }
      fallbackOccurrence++;
    }
  }
  return fallback;
}"#;
