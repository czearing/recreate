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
      'opacity','transform','transformOrigin','translate','rotate','scale',
      'filter','clipPath','maskImage','backgroundColor','backgroundImage','color'
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
    const lastText = new Map();
    const recordedText = new Set();
    const textValue = element => (element.textContent || '').replace(/\s+/g, ' ').trim();
    const trackableText = element => {
      if (!(element instanceof Element)) return '';
      const value = textValue(element);
      if (!value || value.length > 160 || element.childElementCount > 6) return '';
      if (element.querySelector('a,button,input,textarea,select,svg,img,video')) return '';
      return value;
    };
    const seedText = element => {
      if (!(element instanceof Element)) return;
      const value = trackableText(element);
      if (value) {
        lastText.set(pathOf(element), value);
      }
      for (const child of element.querySelectorAll('*')) {
        const childValue = trackableText(child);
        if (childValue) {
          lastText.set(pathOf(child), childValue);
        }
      }
    };
    for (const element of document.querySelectorAll('*')) {
      if (element.parentElement === document.documentElement) seedText(element);
    }
    const recordText = (element, now) => {
      const current = trackableText(element);
      if (!current) return false;
      const target = pathOf(element);
      const before = lastText.get(target);
      lastText.set(target, current);
      if (!before || before === current) return false;
      if (!recordedText.has(target)) {
        recordedText.add(target);
        window.__recreateAttributeMutations.push({
          target, attribute: 'textContent', value: before, time: 0
        });
      }
      window.__recreateAttributeMutations.push({
        target, attribute: 'textContent', value: current, time: now
      });
      return true;
    };
    new MutationObserver(mutations => {
      const now = performance.now() - start;
      for (const mutation of mutations) {
        if (mutation.type === 'characterData') {
          recordText(mutation.target.parentElement, now);
          continue;
        }
        if (mutation.type === 'childList') {
          if (recordText(mutation.target, now)) continue;
          for (const node of mutation.addedNodes) {
            if (!(node instanceof Element)) continue;
            recordText(node, now);
            for (const child of node.querySelectorAll('*')) recordText(child, now);
          }
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
        const rect = element.getBoundingClientRect();
        const value = {
          offset: Math.min(1, (now - start) / 12000),
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
      }
      fullSample = false;
      if (now - start < 12000) {
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
        assert!(super::SOURCE.contains("const lastText = new Map()"));
        assert!(super::SOURCE.contains("const trackableText = element"));
        assert!(super::SOURCE.contains("if (recordText(mutation.target, now)) continue"));
        assert!(
            super::SOURCE.contains("for (const child of node.querySelectorAll('*')) recordText")
        );
        assert!(super::SOURCE.contains("mutation.type === 'characterData'"));
        assert!(super::SOURCE.contains("for (const node of mutation.addedNodes)"));
    }

    #[test]
    fn records_full_recurring_visual_trajectory() {
        for property in [
            "backgroundColor",
            "clipPath",
            "filter",
            "height",
            "maskImage",
            "scale",
            "transformOrigin",
            "width",
        ] {
            assert!(super::SOURCE.contains(property), "missing {property}");
        }
        assert!(super::SOURCE.contains("now - start < 12000"));
    }
}
