pub const SOURCE: &str = r#"
(() => {
  if (window.__recreateLifecycleInstalled) return;
  window.__recreateLifecycleInstalled = true;
  window.__recreateLifecycleAnimations = [];
  window.__recreateAttributeMutations = [];
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
  const record = () => {
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
    const trackedAttributes = new Set(['placeholder','title','aria-label','value']);
    const lastAttribute = new Map();
    const lastText = new WeakMap();
    const recordedText = new WeakSet();
    const textValue = element => (element.textContent || '').replace(/\s+/g, ' ').trim();
    const seedText = element => {
      if (!(element instanceof Element)) return;
      const value = textValue(element);
      if (element.childElementCount === 0 && value && value.length <= 160) {
        lastText.set(element, value);
      }
      for (const child of element.querySelectorAll('*')) {
        const childValue = textValue(child);
        if (child.childElementCount === 0 && childValue && childValue.length <= 160) {
          lastText.set(child, childValue);
        }
      }
    };
    for (const element of document.querySelectorAll('*')) {
      if (element.parentElement === document.documentElement) seedText(element);
    }
    const recordText = (element, now) => {
      if (!(element instanceof Element) || element.childElementCount !== 0) return;
      const current = textValue(element);
      if (!current || current.length > 160) return;
      const before = lastText.get(element);
      lastText.set(element, current);
      if (!before || before === current) return;
      const target = pathOf(element);
      if (!recordedText.has(element)) {
        recordedText.add(element);
        window.__recreateAttributeMutations.push({
          target, attribute: 'textContent', value: before, time: 0
        });
      }
      window.__recreateAttributeMutations.push({
        target, attribute: 'textContent', value: current, time: now
      });
    };
    new MutationObserver(mutations => {
      const now = performance.now() - start;
      for (const mutation of mutations) {
        if (mutation.type === 'characterData') {
          recordText(mutation.target.parentElement, now);
          continue;
        }
        if (mutation.type === 'childList') {
          recordText(mutation.target, now);
          for (const node of mutation.addedNodes) seedText(node);
          continue;
        }
        if (mutation.type !== 'attributes' || !trackedAttributes.has(mutation.attributeName)) continue;
        const element = mutation.target;
        const key = `${pathOf(element)}|${mutation.attributeName}`;
        const current = element.getAttribute(mutation.attributeName) || '';
        if (!lastAttribute.has(key) && mutation.oldValue) {
          window.__recreateAttributeMutations.push({
            target: pathOf(element), attribute: mutation.attributeName,
            value: mutation.oldValue, time: 0
          });
        }
        if (lastAttribute.get(key) !== current) {
          lastAttribute.set(key, current);
          window.__recreateAttributeMutations.push({
            target: pathOf(element), attribute: mutation.attributeName,
            value: current, time: now
          });
        }
      }
    }).observe(document.documentElement, {
      attributes: true, attributeOldValue: true, childList: true,
      characterData: true, characterDataOldValue: true, subtree: true
    });
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
        const style = getComputedStyle(element);
        const value = {
          offset: Math.min(1, (now - start) / 2500),
          opacity: style.opacity,
          transform: style.transform
        };
        const before = previous.get(element);
        previous.set(element, value);
        if (!before) continue;
        const changed = ['opacity','transform']
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
  };
  if (document.readyState === 'loading') {
    addEventListener('DOMContentLoaded', record, { once: true });
  } else {
    record();
  }
})()
"#;

#[cfg(test)]
mod tests {
    #[test]
    fn records_rotating_text_content() {
        assert!(super::SOURCE.contains("attribute: 'textContent'"));
        assert!(super::SOURCE.contains("mutation.type === 'characterData'"));
        assert!(super::SOURCE.contains("for (const node of mutation.addedNodes) seedText(node)"));
    }
}
