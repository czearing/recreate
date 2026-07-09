(() => {
  if (window.__lifecycleAnimationCapture) return;
  const tracks = new Map();
  const animationDefinitions = new Map();
  const changed = new Set();
  let frame = 0;
  let firstTime = null;
  let lastTime = null;
  let loadedAt = null;
  let stopped = false;
  let observer;
  let rafId;

  const pathFor = (element) => {
    const parts = [];
    let current = element;
    while (current?.nodeType === Node.ELEMENT_NODE) {
      const siblings = current.parentElement
        ? [...current.parentElement.children].filter(
            (item) => item.tagName === current.tagName,
          )
        : [current];
      parts.unshift(
        `${current.tagName.toLowerCase()}:nth-of-type(${siblings.indexOf(current) + 1})`,
      );
      current = current.parentElement;
    }
    return `doc(0)>${parts.join('>')}`;
  };

  const stateFor = (element) => {
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    return {
      path: pathFor(element),
      tag: element.tagName.toLowerCase(),
      text: (element.textContent || '').trim().slice(0, 120),
      className: element.getAttribute('class'),
      inlineStyle: element.getAttribute('style'),
      data: Object.fromEntries(
        [...element.attributes]
          .filter((attribute) => attribute.name.startsWith('data-'))
          .map((attribute) => [attribute.name, attribute.value]),
      ),
      rect: {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
      },
      style: {
        display: style.display,
        visibility: style.visibility,
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
        color: style.color,
      },
      media:
        element instanceof HTMLVideoElement
          ? {
              type: 'video',
              currentTime: element.currentTime,
              duration: element.duration,
              paused: element.paused,
              src: element.currentSrc || element.src,
            }
          : element instanceof HTMLCanvasElement
            ? {
                type: 'canvas',
                width: element.width,
                height: element.height,
              }
            : null,
    };
  };

  const track = (element, now) => {
    if (
      !element?.isConnected ||
      ['SCRIPT', 'STYLE', 'META', 'LINK', 'NOSCRIPT'].includes(element.tagName)
    ) {
      return;
    }
    const state = stateFor(element);
    const serialized = JSON.stringify(state);
    let entry = tracks.get(state.path);
    if (!entry) {
      entry = {
        path: state.path,
        tag: state.tag,
        data: state.data,
        samples: [],
        previous: null,
      };
      tracks.set(state.path, entry);
    }
    if (entry.previous === serialized) return;
    entry.previous = serialized;
    entry.samples.push({
      frame,
      time: now,
      text: state.text,
      className: state.className,
      inlineStyle: state.inlineStyle,
      rect: state.rect,
      style: state.style,
      media: state.media,
    });
  };

  const finish = () => {
    if (stopped) return;
    stopped = true;
    observer?.disconnect();
    if (rafId) cancelAnimationFrame(rafId);
  };

  const start = () => {
    const hasLifecycleData = (element) =>
      element.matches('canvas,video') ||
      [...element.attributes].some((attribute) =>
        attribute.name.startsWith('data-'),
      );
    observer = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (mutation.type === 'attributes') {
          changed.add(mutation.target);
        } else if (mutation.type === 'characterData') {
          if (mutation.target.parentElement) {
            changed.add(mutation.target.parentElement);
          }
        } else if (
          mutation.type === 'childList' &&
          mutation.target.nodeType === Node.ELEMENT_NODE &&
          document.readyState !== 'loading' &&
          mutation.target.children.length === 0
        ) {
          changed.add(mutation.target);
        }
        for (const node of mutation.addedNodes || []) {
          if (node.nodeType !== Node.ELEMENT_NODE) continue;
          changed.add(node);
          node
            .querySelectorAll('*')
            .forEach((element) => hasLifecycleData(element) && changed.add(element));
        }
      }
    });
    observer.observe(document, {
      attributes: true,
      attributeFilter: ['class', 'style', 'hidden'],
      attributeOldValue: true,
      characterData: true,
      childList: true,
      subtree: true,
    });
    document
      .querySelectorAll('*')
      .forEach((element) => hasLifecycleData(element) && changed.add(element));

    const tick = (now) => {
      frame++;
      firstTime ??= now;
      lastTime = now;
      const animations = document.getAnimations();
      for (const animation of animations) {
        const target = animation.effect?.target;
        if (!target) continue;
        changed.add(target);
        const path = pathFor(target);
        if (!animationDefinitions.has(path)) {
          animationDefinitions.set(path, {
            path,
            timing: animation.effect?.getTiming?.(),
            keyframes: animation.effect?.getKeyframes?.(),
            playbackRate: animation.playbackRate,
          });
        }
      }
      document
        .querySelectorAll('video:not([paused]),canvas')
        .forEach((element) => changed.add(element));
      for (const element of changed) track(element, now);
      changed.clear();
      const loaded =
        document.documentElement.classList.contains('is-loaded') ||
        (document.readyState === 'complete' &&
          !document.documentElement.classList.contains('is-loading'));
      if (loaded && loadedAt == null) loadedAt = now;
      if (loadedAt != null && now - loadedAt >= 250) {
        finish();
      } else {
        rafId = requestAnimationFrame(tick);
      }
    };
    rafId = requestAnimationFrame(tick);
  };

  if (document.documentElement) {
    start();
  } else {
    new MutationObserver((_, initialObserver) => {
      if (!document.documentElement) return;
      initialObserver.disconnect();
      start();
    }).observe(document, { childList: true });
  }

  Object.defineProperty(window, '__lifecycleAnimationCapture', {
    value: {
      get stopped() {
        return stopped;
      },
      exportAndStop() {
        finish();
        return {
          durationMs: firstTime == null ? 0 : lastTime - firstTime,
          frameCount: frame,
          tracks: [...tracks.values()].map(({ previous, ...entry }) => entry),
          animationDefinitions: [...animationDefinitions.values()],
        };
      },
    },
  });
})();
