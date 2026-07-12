export const discoverHoverExpression = (match) => `(() => {
  const query = ${JSON.stringify(String(match || '').toLowerCase())};
  const roots = [document];
  const candidates = [];
  while (roots.length) {
    const root = roots.shift();
    candidates.push(...root.querySelectorAll(
      'button,a,[role="button"],[data-testid],[tabindex]:not([tabindex="-1"])'
    ));
    for (const element of root.querySelectorAll('*')) {
      if (element.shadowRoot) roots.push(element.shadowRoot);
    }
  }
  const root = candidates
    .map(element => {
      const rect = element.getBoundingClientRect();
      const label = (
        element.getAttribute('aria-label') ||
        element.innerText ||
        element.textContent ||
        ''
      ).trim().toLowerCase();
      return { element, rect, label };
    })
    .filter(candidate =>
      candidate.rect.width > 20 &&
      candidate.rect.height > 20 &&
      (!query || candidate.label.includes(query))
    )
    .sort((left, right) =>
      Number(right.label === query) - Number(left.label === query) ||
      right.rect.width * right.rect.height - left.rect.width * left.rect.height
    )[0]?.element;
  if (!root) return null;
  const scrollPositions = [];
  let current = root;
  while (current) {
    let scrollParent = current.parentElement;
    if (!scrollParent) {
      const currentRoot = current.getRootNode();
      if (currentRoot instanceof ShadowRoot) {
        scrollParent = currentRoot.host;
      }
    }
    if (!scrollParent) break;
    if (
      scrollParent.scrollHeight > scrollParent.clientHeight ||
      scrollParent.scrollWidth > scrollParent.clientWidth
    ) {
      scrollPositions.push({
        element: scrollParent,
        left: scrollParent.scrollLeft,
        top: scrollParent.scrollTop
      });
    }
    current = scrollParent;
  }
  window.__siteSpecHoverWindowScroll = { x: scrollX, y: scrollY };
  window.__siteSpecHoverScrollPositions = scrollPositions;
  root.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' });
  let previousFocus = document.activeElement;
  while (previousFocus?.shadowRoot?.activeElement) {
    previousFocus = previousFocus.shadowRoot.activeElement;
  }
  window.__siteSpecHoverRoot = root;
  window.__siteSpecHoverPreviousFocus = previousFocus;
  const rect = root.getBoundingClientRect();
  return {
    label: (
      root.getAttribute('aria-label') ||
      root.innerText ||
      root.textContent ||
      ''
    ).trim().slice(0, 300),
    tag: root.tagName.toLowerCase(),
    x: rect.x + rect.width / 2,
    y: rect.y + rect.height / 2
  };
})()`;

export const hoverStateExpression = `(() => {
  const root = window.__siteSpecHoverRoot;
  if (!root) return null;
  const rect = root.getBoundingClientRect();
  const style = getComputedStyle(root);
  return {
    rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
    style: {
      opacity: style.opacity,
      transform: style.transform,
      filter: style.filter,
      boxShadow: style.boxShadow,
      backgroundColor: style.backgroundColor
    },
    scroll: {
      x: scrollX,
      y: scrollY,
      containers: (window.__siteSpecHoverScrollPositions || []).map(position => ({
        left: position.element.scrollLeft,
        top: position.element.scrollTop
      }))
    },
    reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches
  };
})()`;

export const prepareHoverExpression = `(() => {
  const root = window.__siteSpecHoverRoot;
  const animations = root.getAnimations({ subtree: true });
  window.__siteSpecHoverBaselineAnimations = new Set(animations);
  window.__siteSpecHoverPausedAnimations = animations
    .filter(animation =>
      !Number.isFinite(animation.effect?.getComputedTiming?.().endTime)
    )
    .map(animation => {
      const state = {
        animation,
        currentTime: animation.currentTime,
        playbackRate: animation.playbackRate,
        playState: animation.playState
      };
      animation.pause();
      return state;
    });
})()`;

export const hoverAnimationExpression = `(() => {
  const root = window.__siteSpecHoverRoot;
  const baseline = window.__siteSpecHoverBaselineAnimations || new Set();
  const relevant = root.getAnimations({ subtree: true })
    .filter(animation => !baseline.has(animation));
  const definitions = relevant.map(animation => ({
    pseudoElement: animation.effect?.pseudoElement || null,
    targetTag: animation.effect?.target?.tagName?.toLowerCase() || null,
    timing: animation.effect?.getTiming?.(),
    keyframes: animation.effect?.getKeyframes?.()
  }));
  relevant.forEach(animation => {
    if (!Number.isFinite(animation.effect?.getComputedTiming?.().endTime)) return;
    try { animation.finish(); } catch {}
  });
  return definitions;
})()`;

export const finishHoverLeaveExpression = `(() => {
  const root = window.__siteSpecHoverRoot;
  const baseline = window.__siteSpecHoverBaselineAnimations || new Set();
  root.getAnimations({ subtree: true })
    .filter(animation => !baseline.has(animation))
    .forEach(animation => {
      if (!Number.isFinite(animation.effect?.getComputedTiming?.().endTime)) return;
      try { animation.finish(); } catch {}
    });
})()`;

export const cleanupHoverExpression = `(() => {
  const previous = window.__siteSpecHoverPreviousFocus;
  if (previous && previous !== document.body) previous.focus();
  for (const state of window.__siteSpecHoverPausedAnimations || []) {
    state.animation.currentTime = state.currentTime;
    state.animation.playbackRate = state.playbackRate;
    if (state.playState === 'running') state.animation.play();
    else state.animation.pause();
  }
  for (const position of window.__siteSpecHoverScrollPositions || []) {
    position.element.scrollLeft = position.left;
    position.element.scrollTop = position.top;
  }
  const windowScroll = window.__siteSpecHoverWindowScroll;
  if (windowScroll) window.scrollTo(windowScroll.x, windowScroll.y);
  const scrollRestored =
    (window.__siteSpecHoverScrollPositions || []).every(position =>
      position.element.scrollLeft === position.left &&
      position.element.scrollTop === position.top
    ) &&
    (!windowScroll || (
      scrollX === windowScroll.x &&
      scrollY === windowScroll.y
    ));
  delete window.__siteSpecHoverRoot;
  delete window.__siteSpecHoverPreviousFocus;
  delete window.__siteSpecHoverScrollPositions;
  delete window.__siteSpecHoverWindowScroll;
  delete window.__siteSpecHoverBaselineAnimations;
  delete window.__siteSpecHoverPausedAnimations;
  return { scrollRestored };
})()`;
