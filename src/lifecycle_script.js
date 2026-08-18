(() => {
  if (window.__recreateLifecycleInstalled) return;
  window.__recreateLifecycleInstalled = true;
__LIFECYCLE_SETTLE__
__LIFECYCLE_SCHEDULED__
  const soonestScheduled = trackScheduled(window);
  window.__recreateLifecycleAnimations = [];
  window.__recreateAttributeMutations = [];  window.__recreateLifecycleDone = false;
  window.__recreatePendingRequests = 0;
  const originalFetch = window.fetch;
  window.fetch = async (...args) => {
    window.__recreatePendingRequests++;
    try { return await originalFetch(...args); }
    finally { window.__recreatePendingRequests--; }
  };
  const originalOpen = XMLHttpRequest.prototype.open;
  const originalSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function(...args) {
    this.__recreateTracked = true;
    return originalOpen.apply(this, args);
  };
  XMLHttpRequest.prototype.send = function(...args) {
    if (this.__recreateTracked) {
      window.__recreatePendingRequests++;
      this.addEventListener('loadend', () => {
        window.__recreatePendingRequests--;
      }, { once: true });
    }
    return originalSend.apply(this, args);
  };
  const record = () => {
    const start = performance.now();
    // Every change this recorder writes is stamped against `start`, so anything that reads
    // those stamps has to be able to reach the same origin or it is measuring a different
    // clock and calling the difference a gap.
    window.__recreateLifecycleStart = start;
    let lastChange = start;
    let longestGap = 0;
    // Whether the page has ever edited itself, which is what makes the next silence a gap.
    let moved = false;
    const previous = new WeakMap();
    const tracks = new Map();
    const safe = new Set([
      'offset','easing','composite','computedOffset',
      'opacity','transform','transformOrigin','translate','rotate','scale',
      'filter','clipPath','maskImage','backgroundColor','backgroundImage','color'
    ]);
    let fullSample = true;
    new MutationObserver(() => {
      fullSample = true;
    }).observe(document.documentElement, {
      attributes: true, childList: true, characterData: true, subtree: true
    });
__NODE_PATH__
__LIFECYCLE_MUTATIONS__
    const sample = () => {
      const now = performance.now();
      const animations = document.getAnimations({ subtree: true });
      const active = animations
        .map(animation => animation.effect?.target)
        .filter(element => element instanceof Element);
      const affectsLayout = animations.some(animation =>
        animation.effect?.getKeyframes?.().some(frame =>
          Object.keys(frame).some(key => !safe.has(key))
        )
      );
      const loading = lifecycleLoading(document);
      const running = animations.map(lifecycleTiming);
      const described = observedTargets(running);
      const elements = fullSample || loading || affectsLayout
        ? document.querySelectorAll('*')
        : new Set(active);
      for (const element of elements) {
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        const value = {
          offset: 0,
          time: now - start,
          x: rect.x,
          y: rect.y,
          width: rect.width,
          height: rect.height,
          opacity: style.opacity,
          transform: style.transform,
          transformOrigin: style.transformOrigin,
          translate: style.translate,
          rotate: style.rotate,
          scale: style.scale,
          clipPath: style.clipPath,
          filter: style.filter,
          maskImage: style.maskImage,
          backgroundColor: style.backgroundColor,
          backgroundImage: style.backgroundImage,
          color: style.color
        };
        const before = previous.get(element);
        previous.set(element, value);
        if (!before) continue;
        const changed = [
          'x','y','width','height','opacity','transform','transformOrigin',
          'translate','rotate','scale','clipPath','filter','maskImage',
          'backgroundColor','backgroundImage','color'
        ]
          .some(key => before[key] !== value[key]);
        if (!changed) continue;
        const path = pathOf(element);
        const frames = tracks.get(path) || [before];
        frames.push(value);
        tracks.set(path, frames);
        if (described.has(element)) continue;
        longestGap = lifecycleGap(longestGap, now - lastChange, moved);
        moved = true;
        lastChange = now;
      }
      fullSample = false;
      const busy = lifecycleBusy(running, loading || lifecycleAwaited(soonestScheduled(), start, moved));
      if (!lifecycleSettled(now - start, now - lastChange, busy, longestGap)) {
        requestAnimationFrame(sample);
      } else {
        // Offsets are resolved against the duration the recorder actually ran for, so a
        // window that closes early still describes the same motion at the same fractions.
        const duration = Math.max(1, now - start);
        window.__recreateLifecycleAnimations = Array.from(tracks, ([target, keyframes]) => ({
          target,
          keyframes: keyframes.map(({ time, ...frame }) => ({
            ...frame,
            offset: Math.min(1, Math.max(0, time / duration))
          })),
          timing: { duration, delay: 0, iterations: 1, easing: 'linear' }
        }));
        window.__recreateLifecycleDone = true;
      }
    };
    requestAnimationFrame(sample);
  };
  if (document.readyState === 'loading') {
    addEventListener('DOMContentLoaded', record, { once: true });
  } else {
    record();
  }
})()
