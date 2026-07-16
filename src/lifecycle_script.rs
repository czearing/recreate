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
      for (const element of document.querySelectorAll('*')) {
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
