pub const SOURCE: &str = r#"
(() => {
  window.__recreateLifecycleAnimations = [];
  window.__recreateLifecycleDone = false;
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
  addEventListener('DOMContentLoaded', () => {
    const start = performance.now();
    const previous = new WeakMap();
    const tracks = new Map();
    const safe = new Set([
      'offset','easing','composite','computedOffset',
      'opacity','transform','filter','clipPath'
    ]);
    let fullSample = true;
    new MutationObserver(() => {
      fullSample = true;
    }).observe(document.documentElement, {
      attributes: true, childList: true, characterData: true, subtree: true
    });
    const pathOf = element => {
      if (element === document.documentElement) return 'html';
      const parts = [];
      for (let node = element; node && node !== document.documentElement; node = node.parentElement) {
        const peers = node.parentElement
          ? Array.from(node.parentElement.children).filter(child => child.tagName === node.tagName)
          : [node];
        parts.push(`${node.tagName.toLowerCase()}:nth-of-type(${peers.indexOf(node) + 1})`);
      }
      return `html>${parts.reverse().join('>')}`;
    };
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
      const loading = document.fonts.status !== 'loaded' ||
        Array.from(document.images).some(image => !image.complete);
      const elements = fullSample || loading || affectsLayout
        ? document.querySelectorAll('*')
        : new Set(active);
      for (const element of elements) {
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        const value = {
          offset: Math.min(1, (now - start) / 2500),
          opacity: style.opacity,
          transform: style.transform,
          x: rect.x,
          y: rect.y,
          width: rect.width,
          height: rect.height
        };
        const before = previous.get(element);
        previous.set(element, value);
        if (!before) continue;
        const changed = ['opacity','transform','x','y','width','height']
          .some(key => before[key] !== value[key]);
        if (!changed) continue;
        const path = pathOf(element);
        const frames = tracks.get(path) || [before];
        frames.push(value);
        tracks.set(path, frames);
      }
      fullSample = false;
      if (now - start < 2500) {
        requestAnimationFrame(sample);
      } else {
        window.__recreateLifecycleAnimations = Array.from(tracks, ([target, keyframes]) => ({
          target,
          keyframes,
          timing: { duration: now - start, delay: 0, iterations: 1, easing: 'linear' }
        }));
        window.__recreateLifecycleDone = true;
      }
    };
    requestAnimationFrame(sample);
  }, { once: true });
})()
"#;
