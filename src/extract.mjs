#!/usr/bin/env node

import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';

const args = Object.fromEntries(
  process.argv.slice(2).map((arg, index, all) => {
    if (!arg.startsWith('--')) return [arg, true];
    const next = all[index + 1];
    return [arg.slice(2), next && !next.startsWith('--') ? next : true];
  }),
);

const url = String(args.url || '');
const match = String(args.match || url);
const requestedTargetId = String(args.target || '');
const outDir = path.resolve(String(args.out || 'site-spec'));
const reuse = Boolean(args.reuse);
const lifecycleRecorderSource = fs.readFileSync(
  new URL('./lifecycle-recorder.js', import.meta.url),
  'utf8',
);
const viewports = String(args.viewports || '1440x900,390x844')
  .split(',')
  .map((value) => {
    const [width, height] = value.split('x').map(Number);
    return { width, height, dpr: 1 };
  });

if (!url && !reuse) {
  throw new Error('Pass --url <url>, or --reuse --match <open-tab substring>.');
}

fs.mkdirSync(outDir, { recursive: true });

const getJson = (pathname) =>
  new Promise((resolve, reject) => {
    http
      .get(`http://localhost:9222${pathname}`, (response) => {
        let body = '';
        response.on('data', (chunk) => (body += chunk));
        response.on('end', () => resolve(JSON.parse(body)));
      })
      .on('error', reject);
  });

class Cdp {
  constructor(ws) {
    this.ws = ws;
    this.id = 0;
    this.pending = new Map();
    this.handlers = new Map();
    ws.addEventListener('message', (event) => {
      const message = JSON.parse(event.data);
      if (message.id && this.pending.has(message.id)) {
        const pending = this.pending.get(message.id);
        this.pending.delete(message.id);
        message.error
          ? pending.reject(new Error(JSON.stringify(message.error)))
          : pending.resolve(message.result);
        return;
      }
      for (const handler of this.handlers.get(message.method) || []) {
        handler(message.params);
      }
    });
    const rejectPending = (event) => {
      const error = new Error(
        event?.message || `CDP socket ${event?.type || 'closed'}`,
      );
      for (const pending of this.pending.values()) pending.reject(error);
      this.pending.clear();
    };
    ws.addEventListener('close', rejectPending);
    ws.addEventListener('error', rejectPending);
  }

  send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++this.id;
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  on(method, handler) {
    const handlers = this.handlers.get(method) || [];
    handlers.push(handler);
    this.handlers.set(method, handlers);
  }

  close() {
    this.ws.close();
  }
}

async function connect(wsUrl) {
  const ws = new WebSocket(wsUrl);
  await new Promise((resolve) => ws.addEventListener('open', resolve));
  return new Cdp(ws);
}

async function findOrOpenTarget() {
  const pages = (await getJson('/json/list')).filter((item) => item.type === 'page');
  if (reuse) {
    const existing = requestedTargetId
      ? pages.find((item) => item.id === requestedTargetId)
      : pages.find((item) => item.url.includes(match));
    if (!existing) throw new Error(`No open page matched: ${match}`);
    return { page: existing, created: false };
  }

  const browser = await connect((await getJson('/json/version')).webSocketDebuggerUrl);
  const { targetId } = await browser.send('Target.createTarget', { url: 'about:blank' });
  for (let attempt = 0; attempt < 20; attempt++) {
    const target = (await getJson('/json/list')).find((item) => item.id === targetId);
    if (target) return { page: target, created: true, browser, targetId };
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error('Created target did not appear in /json/list.');
}

const { page, created, browser, targetId } = await findOrOpenTarget();
const requestedUrl = url || page.url;
const cdp = await connect(page.webSocketDebuggerUrl);
const styleSheets = new Map();
const scripts = new Map();
let latestDocumentResponse;
let latestDocumentBody;
let mainFrameId;

cdp.on('CSS.styleSheetAdded', ({ header }) => styleSheets.set(header.styleSheetId, header));
cdp.on('CSS.styleSheetRemoved', ({ styleSheetId }) => styleSheets.delete(styleSheetId));
cdp.on('Debugger.scriptParsed', (script) => scripts.set(script.scriptId, script));
cdp.on('Network.responseReceived', (event) => {
  if (event.type === 'Document' && event.frameId === mainFrameId) {
    latestDocumentResponse = event;
  }
});
cdp.on('Network.loadingFinished', async ({ requestId }) => {
  if (requestId !== latestDocumentResponse?.requestId) return;
  try {
    latestDocumentBody = await cdp.send('Network.getResponseBody', { requestId });
  } catch (error) {
    latestDocumentBody = { error: String(error) };
  }
});

for (const domain of [
  'Page.enable',
  'Runtime.enable',
  'DOM.enable',
  'CSS.enable',
  'DOMSnapshot.enable',
  'Network.enable',
  'Accessibility.enable',
  'Debugger.enable',
]) {
  await cdp.send(domain);
}
mainFrameId = (await cdp.send('Page.getFrameTree')).frameTree.frame.id;

const computedProperties = [
  'display',
  'position',
  'inset',
  'top',
  'right',
  'bottom',
  'left',
  'box-sizing',
  'width',
  'height',
  'min-width',
  'max-width',
  'min-height',
  'max-height',
  'margin',
  'padding',
  'gap',
  'row-gap',
  'column-gap',
  'flex',
  'flex-grow',
  'flex-shrink',
  'flex-basis',
  'flex-direction',
  'flex-wrap',
  'justify-content',
  'align-items',
  'align-self',
  'order',
  'grid-template-columns',
  'grid-template-rows',
  'grid-auto-flow',
  'overflow',
  'z-index',
  'color',
  'background-color',
  'background-image',
  'background-size',
  'background-position',
  'background-repeat',
  'border',
  'border-radius',
  'box-shadow',
  'opacity',
  'filter',
  'transform',
  'transform-origin',
  'font-family',
  'font-size',
  'font-weight',
  'font-style',
  'line-height',
  'letter-spacing',
  'text-align',
  'text-transform',
  'white-space',
  'object-fit',
  'object-position',
  'cursor',
  'pointer-events',
  'transition',
  'animation',
  'mask-image',
  'clip-path',
];

const supplementFunction = String.raw`async ({ computedProperties }) => {
  const pathFor = (element) => {
    const parts = [];
    let current = element;
    while (current && current.nodeType === Node.ELEMENT_NODE) {
      const root = current.getRootNode();
      const siblings = [...current.parentElement?.children || []].filter(
        (item) => item.tagName === current.tagName,
      );
      const index = Math.max(0, siblings.indexOf(current)) + 1;
      parts.unshift(current.tagName.toLowerCase() + ':nth-of-type(' + index + ')');
      if (root instanceof ShadowRoot) {
        current = root.host;
        parts.unshift('::shadow');
      } else {
        current = current.parentElement;
      }
    }
    return 'doc(0)>' + parts.join('>');
  };
  const styleFor = (element, pseudo) => {
    const style = getComputedStyle(element, pseudo);
    return Object.fromEntries(computedProperties.map((property) => [property, style.getPropertyValue(property)]));
  };
  const assetFor = (element) => {
    const tag = element.tagName.toLowerCase();
    if (tag === 'svg') return { type: 'inline-svg', value: element.outerHTML, path: pathFor(element) };
    if (tag === 'img') return {
      type: 'image',
      path: pathFor(element),
      src: element.getAttribute('src'),
      currentSrc: element.currentSrc,
      srcset: element.getAttribute('srcset'),
      sizes: element.getAttribute('sizes'),
      naturalWidth: element.naturalWidth,
      naturalHeight: element.naturalHeight,
    };
    if (tag === 'video' || tag === 'audio') return {
      type: tag,
      path: pathFor(element),
      src: element.currentSrc || element.getAttribute('src'),
      poster: element.getAttribute('poster'),
    };
    if (tag === 'canvas') {
      try {
        return { type: 'canvas', path: pathFor(element), dataUrl: element.toDataURL(), width: element.width, height: element.height };
      } catch (error) {
        return { type: 'canvas', path: pathFor(element), error: String(error), width: element.width, height: element.height };
      }
    }
    return undefined;
  };
  const assetElements = [];
  const visitAssets = (root) => {
    for (const element of root.querySelectorAll('svg,img,video,audio,canvas')) {
      assetElements.push(element);
    }
    for (const element of root.querySelectorAll('*')) {
      if (element.shadowRoot) visitAssets(element.shadowRoot);
    }
  };
  visitAssets(document);

  const resources = performance.getEntriesByType('resource').map((entry) => ({
    url: entry.name,
    initiatorType: entry.initiatorType,
    transferSize: entry.transferSize,
    decodedBodySize: entry.decodedBodySize,
  }));
  const fonts = [];
  document.fonts.forEach((font) => fonts.push({
    family: font.family,
    style: font.style,
    weight: font.weight,
    stretch: font.stretch,
    status: font.status,
  }));
  const animations = document.getAnimations({ subtree: true }).map((animation) => {
    const effect = animation.effect;
    return {
      targetPath: effect?.target ? pathFor(effect.target) : undefined,
      playState: animation.playState,
      currentTime: animation.currentTime,
      playbackRate: animation.playbackRate,
      timing: effect?.getTiming?.(),
      keyframes: effect?.getKeyframes?.(),
    };
  });
  const meta = [...document.querySelectorAll('meta')].map((element) => ({
    name: element.name,
    property: element.getAttribute('property'),
    content: element.content,
  }));
  const animationElements = [...document.querySelectorAll('*')]
    .filter((element) =>
      [...element.attributes].some((attribute) =>
        /^data-(animation|start|end|scrub|trigger|speed|scroll|stop-at|start-at|parallax|pin)/.test(attribute.name),
      ),
    )
    .map((element) => ({
      path: pathFor(element),
      tag: element.tagName.toLowerCase(),
      text: (element.innerText || '').replace(/\s+/g, ' ').trim().slice(0, 300),
      rect: element.getBoundingClientRect().toJSON(),
      data: Object.fromEntries(
        [...element.attributes]
          .filter((attribute) => attribute.name.startsWith('data-'))
          .map((attribute) => [attribute.name, attribute.value]),
      ),
      style: styleFor(element),
    }));
  const smoothScroll = window.lenis?.scroll
    ? {
        implementation: 'lenis-compatible',
        scroll: window.lenis.scroll.scroll,
        actualScroll: window.lenis.scroll.actualScroll,
        targetScroll: window.lenis.scroll.targetScroll,
        animatedScroll: window.lenis.scroll.animatedScroll,
        limit: window.lenis.scroll.limit,
        velocity: window.lenis.scroll.velocity,
        direction: window.lenis.scroll.direction,
        progress: window.lenis.scroll.progress,
        isScrolling: window.lenis.scroll.isScrolling,
        isStopped: window.lenis.scroll.isStopped,
        options: {
          smoothWheel: window.lenis.scroll.options?.smoothWheel,
          syncTouch: window.lenis.scroll.options?.syncTouch,
          syncTouchLerp: window.lenis.scroll.options?.syncTouchLerp,
          touchInertiaMultiplier:
            window.lenis.scroll.options?.touchInertiaMultiplier,
          easing: window.lenis.scroll.options?.easing?.toString(),
          lerp: window.lenis.scroll.options?.lerp,
          infinite: window.lenis.scroll.options?.infinite,
          gestureOrientation:
            window.lenis.scroll.options?.gestureOrientation,
          orientation: window.lenis.scroll.options?.orientation,
          touchMultiplier: window.lenis.scroll.options?.touchMultiplier,
          wheelMultiplier: window.lenis.scroll.options?.wheelMultiplier,
          autoResize: window.lenis.scroll.options?.autoResize,
          overscroll: window.lenis.scroll.options?.overscroll,
          wrapperPath: window.lenis.scroll.options?.wrapper
            ? pathFor(window.lenis.scroll.options.wrapper)
            : null,
          contentPath: window.lenis.scroll.options?.content
            ? pathFor(window.lenis.scroll.options.content)
            : null,
        },
      }
    : null;
  const horizontalTracks = [...document.querySelectorAll('*')]
    .filter((element) => {
      const style = getComputedStyle(element);
      const children = [...element.children];
      const rect = element.getBoundingClientRect();
      if (
        !children.length ||
        rect.width < 100 ||
        rect.height < 40 ||
        !['div', 'section', 'main', 'ul', 'ol'].includes(
          element.tagName.toLowerCase(),
        )
      ) {
        return false;
      }
      const minLeft = children.length
        ? Math.min(...children.map((child) => child.getBoundingClientRect().left))
        : 0;
      const maxRight = children.length
        ? Math.max(...children.map((child) => child.getBoundingClientRect().right))
        : 0;
      return (
        /(auto|scroll|hidden|clip)/.test(style.overflowX) &&
        (element.scrollWidth > element.clientWidth + 4 ||
          (children.length > 1 &&
            maxRight - minLeft > rect.width + 4))
      );
    })
    .slice(0, 200)
    .map((element) => ({
      path: pathFor(element),
      rect: element.getBoundingClientRect().toJSON(),
      clientWidth: element.clientWidth,
      scrollWidth: element.scrollWidth,
      scrollLeft: element.scrollLeft,
      overflowX: getComputedStyle(element).overflowX,
      transform: getComputedStyle(element).transform,
      childPaths: [...element.children].map(pathFor),
    }));

  return {
    document: {
      url: location.href,
      title: document.title,
      lang: document.documentElement.lang,
      viewport: { width: innerWidth, height: innerHeight, dpr: devicePixelRatio },
      scroll: { width: document.documentElement.scrollWidth, height: document.documentElement.scrollHeight },
      bodyStyle: styleFor(document.body),
      rootStyle: styleFor(document.documentElement),
      meta,
    },
    resources,
    fonts,
    animations,
    animationElements,
    smoothScroll,
    horizontalTracks,
    exactAssets: assetElements
      .slice(0, 5000)
      .map(assetFor)
      .filter(Boolean),
  };
}`;

const sparseValue = (sparse, index) => {
  if (!sparse?.index) return undefined;
  const offset = sparse.index.indexOf(index);
  if (offset < 0) return undefined;
  return sparse.value ? sparse.value[offset] : true;
};

const scrollSweepFunction = String.raw`async () => {
  const pathFor = (element) => {
    if (element === document.scrollingElement) return 'doc(0)>#document-scroll';
    const parts = [];
    let current = element;
    while (current && current.nodeType === Node.ELEMENT_NODE) {
      const root = current.getRootNode();
      const siblings = [...current.parentElement?.children || []].filter(
        (item) => item.tagName === current.tagName,
      );
      const index = Math.max(0, siblings.indexOf(current)) + 1;
      parts.unshift(current.tagName.toLowerCase() + ':nth-of-type(' + index + ')');
      if (root instanceof ShadowRoot) {
        current = root.host;
        parts.unshift('::shadow');
      } else {
        current = current.parentElement;
      }
    }
    return 'doc(0)>' + parts.join('>');
  };
  const allElements = () => {
    const elements = [];
    const visit = (root) => {
      elements.push(...root.querySelectorAll('*'));
      for (const element of root.querySelectorAll('*')) {
        if (element.shadowRoot) visit(element.shadowRoot);
      }
    };
    visit(document);
    return elements;
  };
  const candidates = [
    document.scrollingElement,
    ...allElements().filter((element) => {
      const style = getComputedStyle(element);
      return (
        element.scrollHeight > element.clientHeight + 4 &&
        /(auto|scroll|overlay)/.test(style.overflowY)
      );
    }),
  ]
    .filter(Boolean)
    .filter((element, index, values) => values.indexOf(element) === index)
    .map((element) => ({
      element,
      path: pathFor(element),
      originalTop: element.scrollTop,
      clientHeight: element.clientHeight,
      scrollHeight: element.scrollHeight,
      maxScroll: Math.max(0, element.scrollHeight - element.clientHeight),
    }))
    .filter((item) => item.maxScroll > 4)
    .sort((left, right) => right.maxScroll - left.maxScroll)
    .slice(0, 8);
  const waitForFrame = () =>
    Promise.race([
      new Promise((resolve) =>
        requestAnimationFrame(() =>
          requestAnimationFrame(() => setTimeout(resolve, 80)),
        ),
      ),
      new Promise((resolve) => setTimeout(resolve, 180)),
    ]);
  const results = [];
  for (const candidate of candidates) {
    const checkpoints = [];
    for (const progress of [0, 0.25, 0.5, 0.75, 1]) {
      candidate.element.scrollTop = Math.round(candidate.maxScroll * progress);
      await waitForFrame();
      const visibleElements = allElements()
        .map((element) => {
          const rect = element.getBoundingClientRect();
          if (
            rect.width <= 0 ||
            rect.height <= 0 ||
            rect.bottom < 0 ||
            rect.top > innerHeight ||
            rect.right < 0 ||
            rect.left > innerWidth
          ) {
            return null;
          }
          const style = getComputedStyle(element);
          const animationOwned = Boolean(
            element.closest(
              '[data-animation],[data-header-theme],[data-scrub],[data-scroll-target]',
            ),
          );
          if (
            style.display === 'none' ||
            (!animationOwned &&
              (style.visibility === 'hidden' || style.opacity === '0'))
          ) {
            return null;
          }
          return {
            path: pathFor(element),
            tag: element.tagName.toLowerCase(),
            role: element.getAttribute('role'),
            label: element.getAttribute('aria-label'),
            className: element.className?.baseVal ?? element.className ?? '',
            data: Object.fromEntries(
              [...element.attributes]
                .filter((attribute) => attribute.name.startsWith('data-'))
                .map((attribute) => [attribute.name, attribute.value]),
            ),
            text: (element.innerText || '').replace(/\s+/g, ' ').trim().slice(0, 500),
            rect: {
              x: rect.x,
              y: rect.y,
              width: rect.width,
              height: rect.height,
              right: rect.right,
              bottom: rect.bottom,
            },
            style: {
              position: style.position,
              transform: style.transform,
              opacity: style.opacity,
              filter: style.filter,
              clipPath: style.clipPath,
              backgroundColor: style.backgroundColor,
              color: style.color,
              fontFamily: style.fontFamily,
              fontSize: style.fontSize,
              fontWeight: style.fontWeight,
              lineHeight: style.lineHeight,
              transition: style.transition,
              animation: style.animation,
              willChange: style.willChange,
            },
            asset:
              element instanceof SVGElement && element.tagName.toLowerCase() === 'svg'
                ? element.outerHTML
                : element instanceof HTMLImageElement
                  ? { currentSrc: element.currentSrc, srcset: element.srcset }
                  : element instanceof HTMLVideoElement
                    ? { currentSrc: element.currentSrc, poster: element.poster }
                    : undefined,
          };
        })
        .filter(Boolean)
        .slice(0, 1500);
      const animations = document
        .getAnimations({ subtree: true })
        .slice(0, 500)
        .map((animation) => {
          const effect = animation.effect;
          let keyframes = [];
          try {
            keyframes = effect?.getKeyframes?.() || [];
          } catch {}
          return {
            target: effect?.target ? pathFor(effect.target) : null,
            id: animation.id,
            playState: animation.playState,
            currentTime:
              animation.currentTime == null ? null : String(animation.currentTime),
            startTime:
              animation.startTime == null ? null : String(animation.startTime),
            playbackRate: animation.playbackRate,
            timeline: animation.timeline?.constructor?.name,
            timing: effect?.getTiming?.(),
            computedTiming: effect?.getComputedTiming?.(),
            keyframes,
          };
        });
      checkpoints.push({
        progress,
        scrollTop: candidate.element.scrollTop,
        scrollHeight: candidate.element.scrollHeight,
        visibleElements,
        animations,
      });
    }
    candidate.element.scrollTop = candidate.originalTop;
    await waitForFrame();
    results.push({
      path: candidate.path,
      clientHeight: candidate.clientHeight,
      initialScrollHeight: candidate.scrollHeight,
      maxScroll: candidate.maxScroll,
      checkpoints,
    });
  }
  return results;
}`;

const scrollCandidateExpression = String.raw`(() => {
  const allElements = () => {
    const elements = [];
    const visit = (root) => {
      elements.push(...root.querySelectorAll('*'));
      for (const element of root.querySelectorAll('*')) {
        if (element.shadowRoot) visit(element.shadowRoot);
      }
    };
    visit(document);
    return elements;
  };
  return [
    document.scrollingElement,
    ...allElements().filter((element) => {
      const style = getComputedStyle(element);
      return (
        element.scrollHeight > element.clientHeight + 4 &&
        /(auto|scroll|overlay)/.test(style.overflowY)
      );
    }),
  ]
    .filter(Boolean)
    .filter((element, index, values) => values.indexOf(element) === index)
    .filter(
      (element) => element.scrollHeight - element.clientHeight > 4,
    )
    .sort(
      (left, right) =>
        right.scrollHeight -
        right.clientHeight -
        (left.scrollHeight - left.clientHeight),
    )
    .slice(0, 8);
})()`;

const scrollDescriptorFunction = String.raw`function () {
  const pathFor = (element) => {
    if (element === document.scrollingElement) return 'doc(0)>#document-scroll';
    const parts = [];
    let current = element;
    while (current && current.nodeType === Node.ELEMENT_NODE) {
      const root = current.getRootNode();
      const siblings = [...current.parentElement?.children || []].filter(
        (item) => item.tagName === current.tagName,
      );
      const index = Math.max(0, siblings.indexOf(current)) + 1;
      parts.unshift(current.tagName.toLowerCase() + ':nth-of-type(' + index + ')');
      if (root instanceof ShadowRoot) {
        current = root.host;
        parts.unshift('::shadow');
      } else {
        current = current.parentElement;
      }
    }
    return 'doc(0)>' + parts.join('>');
  };
  return {
    path: pathFor(this),
    originalTop: this.scrollTop,
    clientHeight: this.clientHeight,
    scrollHeight: this.scrollHeight,
    maxScroll: Math.max(0, this.scrollHeight - this.clientHeight),
  };
}`;

const scrollCheckpointFunction = String.raw`function () {
  const pathFor = (element) => {
    const parts = [];
    let current = element;
    while (current && current.nodeType === Node.ELEMENT_NODE) {
      const root = current.getRootNode();
      const siblings = [...current.parentElement?.children || []].filter(
        (item) => item.tagName === current.tagName,
      );
      const index = Math.max(0, siblings.indexOf(current)) + 1;
      parts.unshift(current.tagName.toLowerCase() + ':nth-of-type(' + index + ')');
      if (root instanceof ShadowRoot) {
        current = root.host;
        parts.unshift('::shadow');
      } else {
        current = current.parentElement;
      }
    }
    return 'doc(0)>' + parts.join('>');
  };
  const allElements = () => {
    const elements = [];
    const visit = (root) => {
      elements.push(...root.querySelectorAll('*'));
      for (const element of root.querySelectorAll('*')) {
        if (element.shadowRoot) visit(element.shadowRoot);
      }
    };
    visit(document);
    return elements;
  };
  const visibleElements = allElements()
    .map((element) => {
      const rect = element.getBoundingClientRect();
      if (
        rect.width <= 0 ||
        rect.height <= 0 ||
        rect.bottom < 0 ||
        rect.top > innerHeight ||
        rect.right < 0 ||
        rect.left > innerWidth
      ) {
        return null;
      }
      const style = getComputedStyle(element);
      if (
        style.display === 'none' ||
        style.visibility === 'hidden' ||
        style.opacity === '0'
      ) {
        return null;
      }
      return {
        path: pathFor(element),
        tag: element.tagName.toLowerCase(),
        role: element.getAttribute('role'),
        label: element.getAttribute('aria-label'),
        text: (element.innerText || '').replace(/\s+/g, ' ').trim().slice(0, 500),
        rect: {
          x: rect.x,
          y: rect.y,
          width: rect.width,
          height: rect.height,
          right: rect.right,
          bottom: rect.bottom,
        },
        style: {
          position: style.position,
          transform: style.transform,
          opacity: style.opacity,
          filter: style.filter,
          clipPath: style.clipPath,
          backgroundColor: style.backgroundColor,
          color: style.color,
          fontFamily: style.fontFamily,
          fontSize: style.fontSize,
          fontWeight: style.fontWeight,
          lineHeight: style.lineHeight,
          transition: style.transition,
          animation: style.animation,
          willChange: style.willChange,
        },
        asset:
          element instanceof SVGElement && element.tagName.toLowerCase() === 'svg'
            ? element.outerHTML
            : element instanceof HTMLImageElement
              ? { currentSrc: element.currentSrc, srcset: element.srcset }
              : element instanceof HTMLVideoElement
                ? { currentSrc: element.currentSrc, poster: element.poster }
                : undefined,
      };
    })
    .filter(Boolean)
    .slice(0, 1500);
  const animations = document
    .getAnimations({ subtree: true })
    .slice(0, 500)
    .map((animation) => {
      const effect = animation.effect;
      let keyframes = [];
      try {
        keyframes = effect?.getKeyframes?.() || [];
      } catch {}
      return {
        target: effect?.target ? pathFor(effect.target) : null,
        id: animation.id,
        playState: animation.playState,
        currentTime:
          animation.currentTime == null ? null : String(animation.currentTime),
        startTime:
          animation.startTime == null ? null : String(animation.startTime),
        playbackRate: animation.playbackRate,
        timeline: animation.timeline?.constructor?.name,
        timing: effect?.getTiming?.(),
        computedTiming: effect?.getComputedTiming?.(),
        keyframes,
      };
    });
  return { visibleElements, animations };
}`;

const matchedRuleMapFunction = String.raw`function (selectedPaths) {
  const selected = new Set(selectedPaths);
  const pathFor = (element) => {
    const parts = [];
    let current = element;
    while (current && current.nodeType === Node.ELEMENT_NODE) {
      const root = current.getRootNode();
      const siblings = [...current.parentElement?.children || []].filter(
        (item) => item.tagName === current.tagName,
      );
      const index = Math.max(0, siblings.indexOf(current)) + 1;
      parts.unshift(current.tagName.toLowerCase() + ':nth-of-type(' + index + ')');
      if (root instanceof ShadowRoot) {
        current = root.host;
        parts.unshift('::shadow');
      } else {
        current = current.parentElement;
      }
    }
    return 'doc(0)>' + parts.join('>');
  };
  const elements = [];
  const visitElements = (root) => {
    for (const element of root.querySelectorAll('*')) {
      const path = pathFor(element);
      if (selected.has(path)) elements.push({ element, path });
      if (element.shadowRoot) visitElements(element.shadowRoot);
    }
  };
  visitElements(document);
  const rules = [];
  const visitRules = (ruleList, context = [], sourceURL = '') => {
    for (const rule of ruleList) {
      if (rule.selectorText) {
        rules.push({
          selector: rule.selectorText,
          declarations: [...rule.style].map((name) => ({
            name,
            value: rule.style.getPropertyValue(name),
            important: rule.style.getPropertyPriority(name) === 'important',
          })),
          context,
          sourceURL,
        });
      } else if (rule.cssRules) {
        visitRules(
          rule.cssRules,
          [...context, rule.conditionText || rule.name || rule.constructor.name],
          sourceURL,
        );
      }
    }
  };
  let blockedStylesheets = 0;
  for (const sheet of document.styleSheets) {
    try {
      visitRules(sheet.cssRules || [], [], sheet.href || '');
    } catch {
      blockedStylesheets++;
    }
  }
  return {
    blockedStylesheets,
    records: elements.map(({ element, path }) => ({
      path,
      matchedRules: rules.filter((rule) => {
        try {
          return element.matches(rule.selector);
        } catch {
          return false;
        }
      }),
    })),
  };
}`;

function decodeSnapshot(snapshot) {
  const strings = snapshot.strings;
  const decodedDocuments = snapshot.documents.map((document, documentIndex) => {
    const nodes = document.nodes;
    const children = Array.from({ length: nodes.nodeName.length }, () => []);
    nodes.parentIndex.forEach((parentIndex, nodeIndex) => {
      if (parentIndex >= 0) children[parentIndex].push(nodeIndex);
    });
    const layoutByNode = new Map();
    document.layout.nodeIndex.forEach((nodeIndex, layoutIndex) => {
      const bounds = document.layout.bounds[layoutIndex] || [0, 0, 0, 0];
      const values = document.layout.styles[layoutIndex] || [];
      layoutByNode.set(nodeIndex, {
        rect: {
          x: bounds[0],
          y: bounds[1],
          width: bounds[2],
          height: bounds[3],
          right: bounds[0] + bounds[2],
          bottom: bounds[1] + bounds[3],
        },
        style: Object.fromEntries(
          computedProperties.map((property, index) => [
            property,
            values[index] == null ? '' : strings[values[index]],
          ]),
        ),
        text:
          document.layout.text?.[layoutIndex] == null
            ? undefined
            : strings[document.layout.text[layoutIndex]],
        paintOrder: document.layout.paintOrders?.[layoutIndex],
      });
    });
    const attributesFor = (nodeIndex) => {
      const values = nodes.attributes[nodeIndex] || [];
      const attributes = {};
      for (let index = 0; index < values.length; index += 2) {
        attributes[strings[values[index]]] = strings[values[index + 1]];
      }
      return attributes;
    };
    const tagFor = (nodeIndex) => strings[nodes.nodeName[nodeIndex]].toLowerCase();
    const siblingIndex = (nodeIndex) => {
      const parent = nodes.parentIndex[nodeIndex];
      if (parent < 0) return 1;
      const tag = tagFor(nodeIndex);
      return (
        children[parent]
          .filter((child) => nodes.nodeType[child] === 1 && tagFor(child) === tag)
          .indexOf(nodeIndex) + 1
      );
    };
    const pathFor = (nodeIndex) => {
      const parts = [];
      let current = nodeIndex;
      while (current >= 0) {
        const type = nodes.nodeType[current];
        if (type === 1) {
          parts.unshift(`${tagFor(current)}:nth-of-type(${siblingIndex(current)})`);
        }
        current = nodes.parentIndex[current];
      }
      return `doc(${documentIndex})>${parts.join('>')}`;
    };
    const fingerprintMemo = new Map();
    const fingerprintFor = (nodeIndex, depth = 0) => {
      const key = `${nodeIndex}:${depth}`;
      if (fingerprintMemo.has(key)) return fingerprintMemo.get(key);
      const attrs = attributesFor(nodeIndex);
      const base = `${tagFor(nodeIndex)}[${attrs.role || ''}:${attrs.type || ''}]`;
      const result =
        depth >= 3
          ? base
          : `${base}(${children[nodeIndex]
              .filter((child) => nodes.nodeType[child] === 1)
              .slice(0, 8)
              .map((child) => fingerprintFor(child, depth + 1))
              .join(',')})`;
      fingerprintMemo.set(key, result);
      return result;
    };
    const decodedNodes = nodes.nodeName.map((_, nodeIndex) => {
      const attrs = attributesFor(nodeIndex);
      const layout = layoutByNode.get(nodeIndex);
      const currentSourceIndex = sparseValue(nodes.currentSourceURL, nodeIndex);
      return {
        path: pathFor(nodeIndex),
        parentPath:
          nodes.parentIndex[nodeIndex] >= 0
            ? pathFor(nodes.parentIndex[nodeIndex])
            : undefined,
        backendNodeId: nodes.backendNodeId[nodeIndex],
        nodeType: nodes.nodeType[nodeIndex],
        tag: tagFor(nodeIndex),
        attrs,
        role: attrs.role,
        ariaLabel: attrs['aria-label'],
        text:
          layout?.text ||
          (nodes.nodeValue[nodeIndex] == null
            ? undefined
            : strings[nodes.nodeValue[nodeIndex]]),
        visible: Boolean(layout && layout.rect.width > 0 && layout.rect.height > 0),
        rect: layout?.rect || {
          x: 0,
          y: 0,
          width: 0,
          height: 0,
          right: 0,
          bottom: 0,
        },
        style: layout?.style || {},
        paintOrder: layout?.paintOrder,
        clickable: Boolean(sparseValue(nodes.isClickable, nodeIndex)),
        pseudoType:
          sparseValue(nodes.pseudoType, nodeIndex) == null
            ? undefined
            : strings[sparseValue(nodes.pseudoType, nodeIndex)],
        shadowRootType:
          sparseValue(nodes.shadowRootType, nodeIndex) == null
            ? undefined
            : strings[sparseValue(nodes.shadowRootType, nodeIndex)],
        currentSourceURL:
          currentSourceIndex == null ? undefined : strings[currentSourceIndex],
        fingerprint: fingerprintFor(nodeIndex),
        childCount: children[nodeIndex].filter(
          (child) => nodes.nodeType[child] === 1,
        ).length,
      };
    });
    const groups = new Map();
    for (const node of decodedNodes) {
      const paths = groups.get(node.fingerprint) || [];
      paths.push(node.path);
      groups.set(node.fingerprint, paths);
    }
    const landmarks = new Set([
      'header',
      'nav',
      'main',
      'section',
      'article',
      'aside',
      'footer',
      'form',
      'dialog',
    ]);
    const componentCandidates = decodedNodes
      .map((node) => {
        const occurrences = groups.get(node.fingerprint) || [];
        const area = node.rect.width * node.rect.height;
        const visualContainer =
          node.visible &&
          node.childCount > 0 &&
          area > 2500 &&
          (node.style['background-color'] !== 'rgba(0, 0, 0, 0)' ||
            node.style['background-image'] !== 'none' ||
            node.style.border?.startsWith('0px') === false ||
            node.style['box-shadow'] !== 'none' ||
            node.style['border-radius'] !== '0px');
        const semantic = landmarks.has(node.tag);
        const role = Boolean(node.role);
        const repeated =
          occurrences.length >= 2 &&
          occurrences.length <= 50 &&
          node.childCount > 0 &&
          area >= 2500;
        const reasons = [
          semantic && 'semantic-landmark',
          role && 'semantic-role',
          visualContainer && 'visual-container',
          repeated && 'repeated-subtree',
        ].filter(Boolean);
        const score =
          (semantic ? 5 : 0) +
          (role ? 4 : 0) +
          (visualContainer ? 3 : 0) +
          (repeated ? 2 : 0) +
          (area >= 10000 ? 1 : 0);
        return { node, occurrences, reasons, score };
      })
      .filter(
        ({ node, score }) =>
          node.nodeType === 1 &&
          node.visible &&
          node.childCount > 0 &&
          score >= 4 &&
          !['html', 'body', 'svg', 'g', 'defs'].includes(node.tag),
      )
      .sort((left, right) => right.score - left.score)
      .slice(0, 120)
      .map(({ node, occurrences, reasons, score }) => ({
        representativePath: node.path,
        backendNodeId: node.backendNodeId,
        occurrencePaths: occurrences,
        count: occurrences.length,
        reasons,
        score,
        fingerprint: node.fingerprint,
      }));
    return {
      documentIndex,
      url: strings[document.documentURL],
      title: strings[document.title],
      contentWidth: document.contentWidth,
      contentHeight: document.contentHeight,
      nodes: decodedNodes,
      componentCandidates,
    };
  });
  return {
    documents: decodedDocuments,
    nodes: decodedDocuments.flatMap((document) => document.nodes),
    componentCandidates: decodedDocuments.flatMap(
      (document) => document.componentCandidates,
    ),
  };
}

async function settlePage() {
  await cdp.send('Runtime.evaluate', {
    expression: `Promise.race([
      Promise.all([
        document.fonts.ready,
        ...[...document.images].map((image) => image.complete ? Promise.resolve() : image.decode().catch(() => {}))
      ]),
      new Promise((resolve) => setTimeout(resolve, 1500))
    ]).then(() => true)`,
    awaitPromise: true,
    returnByValue: true,
  });
  await new Promise((resolve) => setTimeout(resolve, 800));
}

async function waitForApplicationReady() {
  const startedAt = Date.now();
  let state = {};
  for (let attempt = 0; attempt < 40; attempt++) {
    try {
      state = (
        await cdp.send('Runtime.evaluate', {
          expression: `({
            readyState: document.readyState,
            fonts: document.fonts.status,
            isLoading: document.documentElement.classList.contains('is-loading'),
            isLoaded: document.documentElement.classList.contains('is-loaded'),
            hasLenisWrapper: Boolean(window.lenis),
            hasLenisScroll: Boolean(window.lenis?.scroll)
          })`,
          returnByValue: true,
        })
      ).result.value;
      if (
        state.readyState === 'complete' &&
        state.fonts === 'loaded' &&
        (!state.isLoading || state.isLoaded) &&
        (!state.hasLenisWrapper || state.hasLenisScroll)
      ) {
        return {
          ready: true,
          waitMs: Date.now() - startedAt,
          state,
        };
      }
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  return {
    ready: false,
    waitMs: Date.now() - startedAt,
    state,
  };
}

const liveScriptSources = new Map();

async function extractViewport(viewport, captureIndex) {
  const timings = {};
  let phaseStart = Date.now();
  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: viewport.dpr,
    mobile: viewport.width < 600,
  });
  await cdp.send('Emulation.setEmulatedMedia', {
    media: 'screen',
    features: [
      { name: 'prefers-reduced-motion', value: 'no-preference' },
      { name: 'prefers-color-scheme', value: 'light' },
    ],
  });
  const lifecycleScript = await cdp.send(
    'Page.addScriptToEvaluateOnNewDocument',
    { source: lifecycleRecorderSource },
  );
  latestDocumentResponse = undefined;
  latestDocumentBody = undefined;
  if (created && captureIndex === 0) {
    await cdp.send('Page.navigate', { url });
  } else {
    await cdp.send('Page.reload', { ignoreCache: false });
  }
  const readiness = await waitForApplicationReady();
  let initialDocument = {
    url: latestDocumentResponse?.response?.url,
    status: latestDocumentResponse?.response?.status,
    mimeType: latestDocumentResponse?.response?.mimeType,
  };
  if (latestDocumentBody && !latestDocumentBody.error) {
    try {
      const documentDir = path.join(outDir, 'documents');
      const filename = `${viewport.width}x${viewport.height}.html`;
      fs.mkdirSync(documentDir, { recursive: true });
      const body = latestDocumentBody.base64Encoded
        ? Buffer.from(latestDocumentBody.body, 'base64')
        : Buffer.from(latestDocumentBody.body);
      fs.writeFileSync(path.join(documentDir, filename), body);
      initialDocument = {
        ...initialDocument,
        file: `documents/${filename}`,
        length: body.length,
      };
    } catch (error) {
      initialDocument.error = String(error);
    }
  } else if (latestDocumentBody?.error) {
    initialDocument.error = latestDocumentBody.error;
  }
  await cdp.send('Runtime.evaluate', {
    expression: `(() => {
      window.lenis?.refresh?.();
      window.lenis?.scroll?.resize?.();
      window.dispatchEvent(new Event('resize'));
    })()`,
  });
  await new Promise((resolve) => setTimeout(resolve, 100));
  await settlePage();
  const lifecycleAnimation = JSON.parse(
    (
      await cdp.send('Runtime.evaluate', {
        expression: `JSON.stringify(
          window.__lifecycleAnimationCapture?.exportAndStop() || {
            durationMs: 0,
            frameCount: 0,
            tracks: [],
            animationDefinitions: []
          }
        )`,
        returnByValue: true,
      })
    ).result.value,
  );
  await cdp.send('Page.removeScriptToEvaluateOnNewDocument', {
    identifier: lifecycleScript.identifier,
  });
  timings.setupAndSettleMs = Date.now() - phaseStart;
  timings.lifecycleAnimationMs = lifecycleAnimation.durationMs;

  phaseStart = Date.now();
  const scrollCandidateResult = await cdp.send('Runtime.evaluate', {
    expression: scrollCandidateExpression,
    returnByValue: false,
  });
  const scrollCandidateProperties = await cdp.send('Runtime.getProperties', {
    objectId: scrollCandidateResult.result.objectId,
    ownProperties: true,
  });
  const scrollCandidateObjectIds = scrollCandidateProperties.result
    .filter((property) => /^\d+$/.test(property.name) && property.value?.objectId)
    .map((property) => property.value.objectId);
  const documentObjectId = (
    await cdp.send('Runtime.evaluate', {
      expression: 'document',
      returnByValue: false,
    })
  ).result.objectId;
  const adaptiveScroll = (
    await cdp.send('Runtime.evaluate', {
      expression: `(() => {
        const current =
          window.lenis?.scroll?.scroll ??
          document.scrollingElement.scrollTop;
        const nativeLimit = Math.max(
          0,
          document.scrollingElement.scrollHeight - innerHeight,
          document.body.scrollHeight - innerHeight
        );
        const limit = Math.max(window.lenis?.scroll?.limit ?? 0, nativeLimit);
        const sections = [...document.querySelectorAll('section')].map((element) => {
          const rect = element.getBoundingClientRect();
          return {
            top: rect.top + current,
            bottom: rect.bottom + current
          };
        });
        return {
          current,
          limit,
          viewportHeight: innerHeight,
          hasLenis: Boolean(window.lenis?.scrollTo && window.lenis?.scroll),
          sections
        };
      })()`,
      returnByValue: true,
    })
  ).result.value;
  const scrollStates = [];
  for (let candidateIndex = 0; candidateIndex < scrollCandidateObjectIds.length; candidateIndex++) {
    const objectId = scrollCandidateObjectIds[candidateIndex];
    const descriptor = (
      await cdp.send('Runtime.callFunctionOn', {
        objectId,
        functionDeclaration: scrollDescriptorFunction,
        returnByValue: true,
      })
    ).result.value;
    const checkpoints = [];
    const maxScroll =
      candidateIndex === 0
        ? Math.max(descriptor.maxScroll, adaptiveScroll.limit)
        : descriptor.maxScroll;
    const positions = new Set([0, maxScroll]);
    if (candidateIndex === 0) {
      const step = Math.max(200, adaptiveScroll.viewportHeight * 0.75);
      for (let top = 0; top < maxScroll; top += step) {
        positions.add(Math.round(top));
      }
      for (const section of adaptiveScroll.sections) {
        for (const value of [
          section.top - adaptiveScroll.viewportHeight,
          section.top,
          section.bottom - adaptiveScroll.viewportHeight,
          section.bottom,
        ]) {
          positions.add(
            Math.round(Math.max(0, Math.min(maxScroll, value))),
          );
        }
      }
    } else {
      for (const progress of [0.25, 0.5, 0.75]) {
        positions.add(Math.round(maxScroll * progress));
      }
    }
    for (const scrollTop of [...positions].sort((left, right) => left - right)) {
      if (candidateIndex === 0 && adaptiveScroll.hasLenis) {
        await cdp.send('Runtime.evaluate', {
          expression: `(() => {
            window.lenis.scrollTo(${scrollTop}, {
              immediate: true,
              duration: 0,
              force: true,
              lock: false
            });
            window.lenis.scroll?.raf?.(performance.now());
          })()`,
        });
      } else {
        await cdp.send('Runtime.callFunctionOn', {
          objectId,
          functionDeclaration: `function (top) { this.scrollTop = top; return this.scrollTop; }`,
          arguments: [{ value: scrollTop }],
          returnByValue: true,
        });
      }
      const checkpoint = (
        await cdp.send('Runtime.callFunctionOn', {
          objectId: documentObjectId,
          functionDeclaration: scrollCheckpointFunction,
          returnByValue: true,
        })
      ).result.value;
      checkpoints.push({
        progress: maxScroll ? scrollTop / maxScroll : 0,
        scrollTop,
        scrollHeight: descriptor.scrollHeight,
        ...checkpoint,
      });
    }
    if (candidateIndex === 0 && adaptiveScroll.hasLenis) {
      await cdp.send('Runtime.evaluate', {
        expression: `window.lenis.scrollTo(${adaptiveScroll.current}, {
          immediate: true,
          duration: 0,
          force: true
        })`,
      });
    } else {
      await cdp.send('Runtime.callFunctionOn', {
        objectId,
        functionDeclaration: `function (top) { this.scrollTop = top; }`,
        arguments: [{ value: descriptor.originalTop }],
      });
    }
    scrollStates.push({
      path: descriptor.path,
      clientHeight: descriptor.clientHeight,
      initialScrollHeight: descriptor.scrollHeight,
      maxScroll,
      checkpoints,
    });
  }
  await settlePage();
  timings.scrollSweepMs = Date.now() - phaseStart;

  phaseStart = Date.now();
  const snapshot = await cdp.send('DOMSnapshot.captureSnapshot', {
    computedStyles: computedProperties,
    includePaintOrder: true,
    includeDOMRects: true,
    includeBlendedBackgroundColors: true,
    includeTextColorOpacities: true,
  });
  const decoded = decodeSnapshot(snapshot);
  timings.snapshotMs = Date.now() - phaseStart;

  phaseStart = Date.now();
  const result = await cdp.send('Runtime.callFunctionOn', {
    objectId: (
      await cdp.send('Runtime.evaluate', {
        expression: 'document',
        returnByValue: false,
      })
    ).result.objectId,
    functionDeclaration: supplementFunction,
    arguments: [{ value: { computedProperties } }],
    awaitPromise: true,
    returnByValue: true,
  });
  const extracted = {
    ...result.result.value,
    scrollStates,
    documents: decoded.documents,
    nodes: decoded.nodes,
    componentCandidates: decoded.componentCandidates,
    readiness,
    lifecycleAnimation,
    initialDocument,
  };
  timings.supplementMs = Date.now() - phaseStart;

  phaseStart = Date.now();
  const descriptorResult = await cdp.send('Runtime.evaluate', {
    expression: `(() => {
      const selector = 'button,a,input,textarea,select,summary,[role],[tabindex],[contenteditable=true]';
      const pathFor = (element) => {
        const parts = [];
        let current = element;
        while (current && current.nodeType === Node.ELEMENT_NODE) {
          const siblings = [...current.parentElement?.children || []].filter((item) => item.tagName === current.tagName);
          parts.unshift(current.tagName.toLowerCase() + ':nth-of-type(' + (Math.max(0, siblings.indexOf(current)) + 1) + ')');
          current = current.parentElement;
        }
        return 'doc(0)>' + parts.join('>');
      };
      const elements = [];
      const visit = (root) => {
        elements.push(...root.querySelectorAll(selector));
        for (const element of root.querySelectorAll('*')) {
          if (element.shadowRoot) visit(element.shadowRoot);
        }
      };
      visit(document);
      return elements.slice(0, 250).map((element) => ({
        path: pathFor(element),
        tag: element.tagName.toLowerCase(),
        role: element.getAttribute('role'),
        href: element.href || null,
        type: element.getAttribute('type'),
        formAction: element.formAction || null,
        disabled: Boolean(element.disabled || element.getAttribute('aria-disabled') === 'true'),
        ariaExpanded: element.getAttribute('aria-expanded'),
        ariaPressed: element.getAttribute('aria-pressed'),
        ariaSelected: element.getAttribute('aria-selected'),
        ariaHaspopup: element.getAttribute('aria-haspopup'),
        label: (element.getAttribute('aria-label') || element.textContent || '').trim().slice(0, 200)
      }));
    })()`,
    returnByValue: true,
  });
  const eventTargets = await cdp.send('Runtime.evaluate', {
    expression: '[window, document]',
    returnByValue: false,
  });
  const eventTargetProperties = await cdp.send('Runtime.getProperties', {
    objectId: eventTargets.result.objectId,
    ownProperties: true,
  });
  const eventTargetObjectIds = eventTargetProperties.result
    .filter((item) => /^\d+$/.test(item.name) && item.value?.objectId)
    .map((item) => item.value.objectId);
  const listenerResults = await Promise.all(
    eventTargetObjectIds.map((objectId, index) =>
      cdp
        .send('DOMDebugger.getEventListeners', {
          objectId,
          depth: index === 1 ? -1 : 1,
          pierce: true,
        })
        .catch(() => ({ listeners: [] })),
    ),
  );
  const listenerFor = (listener, target) => ({
    target,
    type: listener.type,
    capture: listener.useCapture,
    passive: listener.passive,
    once: listener.once,
    scriptId: listener.scriptId,
    lineNumber: listener.lineNumber,
    columnNumber: listener.columnNumber,
    backendNodeId: listener.backendNodeId,
  });
  const allListeners = listenerResults.flatMap((result, index) =>
    result.listeners.map((listener) =>
      listenerFor(listener, index === 0 ? 'window' : 'document'),
    ),
  );
  await Promise.all(
    [...new Set(allListeners.map((listener) => listener.scriptId))]
      .filter((scriptId) => scriptId && scriptId !== '0')
      .map(async (scriptId) => {
        if (liveScriptSources.has(scriptId)) return;
        try {
          const source = (
            await cdp.send('Debugger.getScriptSource', { scriptId })
          ).scriptSource;
          liveScriptSources.set(scriptId, {
            source,
            parsed: scripts.get(scriptId),
          });
        } catch {}
      }),
  );
  const listenersByBackendId = new Map();
  for (const listener of allListeners.filter(
    (listener) => listener.backendNodeId,
  )) {
    const listeners = listenersByBackendId.get(listener.backendNodeId) || [];
    listeners.push(listener);
    listenersByBackendId.set(listener.backendNodeId, listeners);
  }
  const nodeByPath = new Map(extracted.nodes.map((node) => [node.path, node]));
  const descriptors = descriptorResult.result.value || [];
  const behaviorByPath = new Map(
    descriptors.map((descriptor) => {
      const sourceNode = nodeByPath.get(descriptor.path);
      return [
        descriptor.path,
        {
          ...descriptor,
          backendNodeId: sourceNode?.backendNodeId,
          listeners:
            listenersByBackendId.get(sourceNode?.backendNodeId) || [],
        },
      ];
    }),
  );
  for (const [backendNodeId, listeners] of listenersByBackendId) {
    const sourceNode = extracted.nodes.find(
      (node) => node.backendNodeId === backendNodeId,
    );
    if (!sourceNode || behaviorByPath.has(sourceNode.path)) continue;
    behaviorByPath.set(sourceNode.path, {
      path: sourceNode.path,
      backendNodeId,
      tag: sourceNode.tag,
      role: sourceNode.role,
      href: sourceNode.attrs?.href,
      type: sourceNode.attrs?.type,
      disabled:
        sourceNode.attrs?.disabled != null ||
        sourceNode.attrs?.['aria-disabled'] === 'true',
      ariaExpanded: sourceNode.attrs?.['aria-expanded'],
      ariaPressed: sourceNode.attrs?.['aria-pressed'],
      ariaSelected: sourceNode.attrs?.['aria-selected'],
      ariaHaspopup: sourceNode.attrs?.['aria-haspopup'],
      label:
        sourceNode.ariaLabel ||
        String(sourceNode.text || '').trim().slice(0, 200),
      listeners,
    });
  }
  const behaviors = [...behaviorByPath.values()];
  extracted.behaviors = behaviors;
  extracted.globalListeners = allListeners.filter(
    (listener) => !listener.backendNodeId,
  );
  timings.behaviorsMs = Date.now() - phaseStart;

  phaseStart = Date.now();
  await cdp.send('DOM.getDocument', { depth: 0, pierce: true });
  const nodeByBackendId = new Map(
    extracted.nodes.map((node) => [node.backendNodeId, node]),
  );
  const deepCandidates = [];
  const seenBackendIds = new Set();
  const addDeepCandidate = (backendNodeId) => {
    if (!backendNodeId || seenBackendIds.has(backendNodeId)) return;
    seenBackendIds.add(backendNodeId);
    deepCandidates.push(backendNodeId);
  };
  extracted.nodes
    .filter(
      (node) =>
        node.visible &&
        (node.clickable ||
          node.role ||
          ['button', 'a', 'input', 'textarea', 'select', 'summary'].includes(
            node.tag,
          )),
    )
    .forEach((node) => addDeepCandidate(node.backendNodeId));
  extracted.componentCandidates
    .slice()
    .sort((left, right) => right.score - left.score)
    .forEach((candidate) => addDeepCandidate(candidate.backendNodeId));

  const selectedBackendIds = deepCandidates.slice(0, 250);
  let frontendNodeIds = [];
  if (selectedBackendIds.length) {
    try {
      frontendNodeIds = (
        await cdp.send('DOM.pushNodesByBackendIdsToFrontend', {
          backendNodeIds: selectedBackendIds,
        })
      ).nodeIds;
    } catch {}
  }
  const deepStyleRecords = new Array(frontendNodeIds.length);
  for (let offset = 0; offset < frontendNodeIds.length; offset += 8) {
    const indexes = frontendNodeIds
      .slice(offset, offset + 8)
      .map((nodeId, batchIndex) => ({ nodeId, index: offset + batchIndex }))
      .filter(({ nodeId }) => nodeId);
    await Promise.all(
      indexes.map(async ({ nodeId, index }) => {
        const sourceNode = nodeByBackendId.get(selectedBackendIds[index]);
        const stateStyles = {};
        for (const pseudoClass of ['hover', 'active', 'focus', 'focus-visible']) {
          try {
            await cdp.send('CSS.forcePseudoState', {
              nodeId,
              forcedPseudoClasses: [pseudoClass],
            });
            const computed = Object.fromEntries(
              (
                await cdp.send('CSS.getComputedStyleForNode', { nodeId })
              ).computedStyle
                .filter((property) => computedProperties.includes(property.name))
                .map((property) => [property.name, property.value]),
            );
            const delta = Object.fromEntries(
              Object.entries(computed).filter(
                ([property, value]) => sourceNode?.style?.[property] !== value,
              ),
            );
            if (Object.keys(delta).length) stateStyles[pseudoClass] = delta;
          } catch {}
        }
        try {
          await cdp.send('CSS.forcePseudoState', {
            nodeId,
            forcedPseudoClasses: [],
          });
        } catch {}
        deepStyleRecords[index] = {
          path: sourceNode?.path,
          backendNodeId: selectedBackendIds[index],
          stateStyles,
          matchedRules: [],
        };
      }),
    );
  }
  extracted.deepStyles = deepStyleRecords.filter(Boolean);
  timings.deepStylesMs = Date.now() - phaseStart;

  phaseStart = Date.now();
  try {
    extracted.accessibility = await cdp.send('Accessibility.getFullAXTree', {});
  } catch (error) {
    extracted.accessibility = { error: String(error) };
  }
  timings.accessibilityMs = Date.now() - phaseStart;
  timings.totalMs = Object.values(timings).reduce(
    (total, duration) => total + duration,
    0,
  );
  extracted.timings = timings;
  return extracted;
}

const captures = [];
for (let captureIndex = 0; captureIndex < viewports.length; captureIndex++) {
  const viewport = viewports[captureIndex];
  console.error(`Capturing ${viewport.width}x${viewport.height}: ${page.url || url}`);
  captures.push(await extractViewport(viewport, captureIndex));
}

async function mapMatchedRules(paths) {
  if (!paths.length) return { blockedStylesheets: 0, records: [] };
  const documentObjectId = (
    await cdp.send('Runtime.evaluate', {
      expression: 'document',
      returnByValue: false,
    })
  ).result.objectId;
  return (
    await cdp.send('Runtime.callFunctionOn', {
      objectId: documentObjectId,
      functionDeclaration: matchedRuleMapFunction,
      arguments: [{ value: paths }],
      returnByValue: true,
    })
  ).result.value;
}

const matchedRulesStartedAt = Date.now();
const lastCapture = captures.at(-1);
const lastPaths = new Set(
  lastCapture?.deepStyles.map((record) => record.path) || [],
);
const lastRuleMap = await mapMatchedRules([...lastPaths]);
const rulesByPath = new Map(
  (lastRuleMap.records || []).map((record) => [
    record.path,
    record.matchedRules,
  ]),
);
const earlierOnlyPaths = [
  ...new Set(
    captures
      .slice(0, -1)
      .flatMap((capture) => capture.deepStyles.map((record) => record.path))
      .filter((pathValue) => !lastPaths.has(pathValue)),
  ),
];
let earlierRuleMap = { blockedStylesheets: 0, records: [] };
if (earlierOnlyPaths.length) {
  const viewport = viewports[0];
  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: viewport.dpr,
    mobile: viewport.width < 600,
  });
  await cdp.send('Page.reload', { ignoreCache: false });
  await waitForApplicationReady();
  earlierRuleMap = await mapMatchedRules(earlierOnlyPaths);
  for (const record of earlierRuleMap.records || []) {
    rulesByPath.set(record.path, record.matchedRules);
  }
}
for (const capture of captures) {
  for (const record of capture.deepStyles) {
    record.matchedRules = rulesByPath.get(record.path) || [];
  }
  capture.cssomBlockedStylesheetCount = Math.max(
    lastRuleMap.blockedStylesheets || 0,
    earlierRuleMap.blockedStylesheets || 0,
  );
}
const matchedRulesMs = Date.now() - matchedRulesStartedAt;

const stylesheetManifest = [];
const stylesheetTextByFile = new Map();
const cssDir = path.join(outDir, 'stylesheets');
fs.mkdirSync(cssDir, { recursive: true });
for (const [styleSheetId, header] of styleSheets) {
  try {
    const { text } = await cdp.send('CSS.getStyleSheetText', { styleSheetId });
    const filename = `${String(stylesheetManifest.length).padStart(4, '0')}.css`;
    fs.writeFileSync(path.join(cssDir, filename), text);
    stylesheetTextByFile.set(`stylesheets/${filename}`, text);
    stylesheetManifest.push({
      file: `stylesheets/${filename}`,
      sourceURL: header.sourceURL,
      origin: header.origin,
      title: header.title,
      disabled: header.disabled,
      isInline: header.isInline,
      startLine: header.startLine,
      startColumn: header.startColumn,
      length: text.length,
      mediaQueries: [
        ...new Set(
          [...text.matchAll(/@media\s+([^{]+)/g)].map((match) => match[1].trim()),
        ),
      ].slice(0, 500),
      keyframes: [
        ...new Set(
          [...text.matchAll(/@(?:-webkit-)?keyframes\s+([^\s{]+)/g)].map(
            (match) => match[1].trim(),
          ),
        ),
      ].slice(0, 500),
      pseudoSelectors: [
        ...new Set(
          [...text.matchAll(/([^{}]+:(?:hover|active|focus-visible|focus|disabled|checked|expanded)[^{}]*)\{/g)]
            .map((match) => match[1].trim())
            .filter((selector) => selector.length < 500),
        ),
      ].slice(0, 1000),
      customProperties: [
        ...new Set(
          [...text.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]),
        ),
      ].slice(0, 2000),
    });
  } catch (error) {
    stylesheetManifest.push({
      sourceURL: header.sourceURL,
      error: String(error),
    });
  }
}

const scriptDir = path.join(outDir, 'scripts');
fs.mkdirSync(scriptDir, { recursive: true });
const usedScriptIds = new Set(
  captures.flatMap((capture) =>
    [
      ...capture.behaviors.flatMap((behavior) => behavior.listeners),
      ...(capture.globalListeners || []),
    ]
      .map((listener) => listener.scriptId)
      .filter(Boolean),
  ),
);
const scriptManifest = [];
const sourceByScriptId = new Map();
for (const scriptId of usedScriptIds) {
  if (scriptId === '0') {
    scriptManifest.push({
      scriptId,
      status: 'protocol-opaque',
      reason: 'CDP reported no script location for this listener.',
    });
    continue;
  }
  const liveScript = liveScriptSources.get(scriptId);
  if (liveScript) {
    const filename = `${scriptId.replace(/[^\w.-]+/g, '_')}.js`;
    fs.writeFileSync(path.join(scriptDir, filename), liveScript.source);
    sourceByScriptId.set(scriptId, {
      source: liveScript.source,
      filename,
      lines: liveScript.source.split(/\r?\n/),
    });
    scriptManifest.push({
      scriptId,
      file: `scripts/${filename}`,
      url: liveScript.parsed?.url,
      startLine: liveScript.parsed?.startLine,
      startColumn: liveScript.parsed?.startColumn,
      length: liveScript.source.length,
      hash: liveScript.parsed?.hash,
      sourceMapURL: liveScript.parsed?.sourceMapURL,
      retrieval: 'debugger-live',
    });
    continue;
  }
  let sourceCdp;
  try {
    sourceCdp = await connect(page.webSocketDebuggerUrl);
    await sourceCdp.send('Debugger.enable');
    const source = (await sourceCdp.send('Debugger.getScriptSource', { scriptId }))
      .scriptSource;
    const filename = `${scriptId.replace(/[^\w.-]+/g, '_')}.js`;
    fs.writeFileSync(path.join(scriptDir, filename), source);
    sourceByScriptId.set(scriptId, {
      source,
      filename,
      lines: source.split(/\r?\n/),
    });
    const parsed = scripts.get(scriptId);
    scriptManifest.push({
      scriptId,
      file: `scripts/${filename}`,
      url: parsed?.url,
      startLine: parsed?.startLine,
      startColumn: parsed?.startColumn,
      length: source.length,
      hash: parsed?.hash,
      sourceMapURL: parsed?.sourceMapURL,
      retrieval: 'debugger',
    });
  } catch (error) {
    const parsed = scripts.get(scriptId);
    try {
      if (!parsed?.url?.startsWith('http')) throw error;
      const response = await fetch(parsed.url);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const source = await response.text();
      const filename = `${scriptId.replace(/[^\w.-]+/g, '_')}.js`;
      fs.writeFileSync(path.join(scriptDir, filename), source);
      sourceByScriptId.set(scriptId, {
        source,
        filename,
        lines: source.split(/\r?\n/),
      });
      scriptManifest.push({
        scriptId,
        file: `scripts/${filename}`,
        url: parsed.url,
        length: source.length,
        hash: parsed.hash,
        sourceMapURL: parsed.sourceMapURL,
        retrieval: 'network-fallback',
        debuggerError: String(error),
      });
    } catch (fallbackError) {
      scriptManifest.push({
        scriptId,
        url: parsed?.url,
        hash: parsed?.hash,
        error: String(fallbackError),
        debuggerError: String(error),
      });
    }
  } finally {
    sourceCdp?.close();
  }
}
for (const capture of captures) {
  const listeners = [
    ...capture.behaviors.flatMap((behavior) => behavior.listeners),
    ...(capture.globalListeners || []),
  ];
  for (const listener of listeners) {
      const script = sourceByScriptId.get(listener.scriptId);
      if (!script) continue;
      const line = script.lines[listener.lineNumber] || '';
      const center = listener.columnNumber || 0;
      listener.sourceFile = `scripts/${script.filename}`;
      listener.sourceExcerpt =
        line.length <= 1200
          ? line
          : line.slice(Math.max(0, center - 600), center + 600);
      listener.sourceStatus = 'captured';
  }
}
for (const capture of captures) {
  const listeners = [
    ...capture.behaviors.flatMap((behavior) => behavior.listeners),
    ...(capture.globalListeners || []),
  ];
  for (const listener of listeners) {
      if (listener.sourceStatus) continue;
      listener.sourceStatus = listener.handlerSource
        ? 'captured'
        : listener.scriptId === '0' || listener.lineNumber < 0
          ? 'protocol-opaque'
          : 'unavailable';
  }
}

const animationTypes = [
  ...new Set(
    captures.flatMap((capture) =>
      (capture.animationElements || [])
        .map((element) => element.data?.['data-animation'])
        .filter(Boolean),
    ),
  ),
];
const escapeRegExp = (value) =>
  value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const animationImplementations = animationTypes.map((type) => {
  const references = [];
  for (const script of sourceByScriptId.values()) {
    const exactPatterns = [`"${type}"`, `'${type}'`];
    const offsets = exactPatterns.flatMap((pattern) => {
      const values = [];
      let offset = 0;
      while ((offset = script.source.indexOf(pattern, offset)) >= 0) {
        values.push(offset);
        offset += pattern.length;
      }
      return values;
    });
    let registration = script.source.match(
      new RegExp(
        `selector:\\s*['"][^'"]*data-animation(?:\\\\\\^)?=["']${escapeRegExp(type)}["'][^'"]*['"]\\s*,\\s*Instance:([A-Za-z_$][\\w$]*)`,
      ),
    );
    if (!registration) {
      const prefixPattern =
        /selector:\s*['"][^'"]*data-animation\^=["']([^"']+)["'][^'"]*['"]\s*,\s*Instance:([A-Za-z_$][\w$]*)/g;
      for (const match of script.source.matchAll(prefixPattern)) {
        if (type.startsWith(match[1])) {
          registration = {
            1: match[2],
            index: match.index,
          };
          break;
        }
      }
    }
    let implementation;
    if (registration) {
      const className = registration[1];
      const classStart = script.source.lastIndexOf(
        `class ${className}`,
        registration.index,
      );
      if (classStart >= 0) {
        const nextClass = script.source.indexOf('class ', classStart + 6);
        const classEnd =
          nextClass > classStart
            ? Math.min(nextClass, classStart + 20000)
            : Math.min(script.source.length, classStart + 20000);
        implementation = {
          className,
          offset: classStart,
          source: script.source.slice(classStart, classEnd),
        };
      }
    }
    if (offsets.length || implementation) {
      references.push({
        sourceFile: `scripts/${script.filename}`,
        offsets: [...new Set(offsets)].slice(0, 20),
        excerpts: [...new Set(offsets)]
          .slice(0, 3)
          .map((offset) =>
            script.source.slice(
              Math.max(0, offset - 1000),
              Math.min(script.source.length, offset + 3000),
            ),
          ),
        implementation,
      });
    }
  }
  const stylesheetReferences = [];
  for (const [sourceFile, text] of stylesheetTextByFile) {
    const offsets = [];
    for (const pattern of [
      `data-animation=${type}`,
      `data-animation="${type}"`,
      `data-animation='${type}'`,
    ]) {
      let offset = 0;
      while ((offset = text.indexOf(pattern, offset)) >= 0) {
        offsets.push(offset);
        offset += pattern.length;
      }
    }
    if (offsets.length) {
      stylesheetReferences.push({
        sourceFile,
        offsets: [...new Set(offsets)].slice(0, 20),
        excerpts: [...new Set(offsets)]
          .slice(0, 3)
          .map((offset) =>
            text.slice(
              Math.max(0, offset - 500),
              Math.min(text.length, offset + 1500),
            ),
          ),
      });
    }
  }
  return { type, references, stylesheetReferences };
});
const animationLibrarySignals = [];
for (const [scriptId, script] of sourceByScriptId) {
  const signals = {};
  for (const term of [
    'gsap',
    'ScrollTrigger',
    'requestAnimationFrame',
    'IntersectionObserver',
    'data-animation',
    'data-scrub',
    'data-scroll-to',
  ]) {
    let count = 0;
    let offset = 0;
    while ((offset = script.source.indexOf(term, offset)) >= 0) {
      count++;
      offset += term.length;
    }
    if (count) signals[term] = count;
  }
  if (Object.keys(signals).length) {
    animationLibrarySignals.push({
      scriptId,
      sourceFile: `scripts/${script.filename}`,
      signals,
    });
  }
}
const dataAttributeNames = [
  ...new Set(
    captures.flatMap((capture) =>
      [
        ...capture.nodes.flatMap((node) =>
          Object.keys(node.attrs || {}).filter((name) =>
            name.startsWith('data-'),
          ),
        ),
        ...(capture.lifecycleAnimation?.tracks || []).flatMap((track) =>
          Object.keys(track.data || {}).filter((name) =>
            name.startsWith('data-'),
          ),
        ),
      ],
    ),
  ),
].slice(0, 500);
const dataAttributeImplementations = dataAttributeNames
  .map((attribute) => {
    const references = [];
    for (const script of sourceByScriptId.values()) {
      const offsets = [];
      let matchedTerm = attribute;
      for (const term of [attribute, attribute.slice(5)]) {
        let offset = 0;
        while ((offset = script.source.indexOf(term, offset)) >= 0) {
          offsets.push(offset);
          offset += term.length;
        }
        if (offsets.length) {
          matchedTerm = term;
          break;
        }
      }
      if (offsets.length) {
        references.push({
          sourceFile: `scripts/${script.filename}`,
          matchedTerm,
          offsets: offsets.slice(0, 50),
          excerpts: offsets.slice(0, 3).map((value) =>
            script.source.slice(
              Math.max(0, value - 800),
              Math.min(script.source.length, value + 2200),
            ),
          ),
        });
      }
    }
    return { attribute, references };
  })
  .filter((item) => item.references.length);

const byPath = new Map();
const componentPaths = new Set(
  captures.flatMap((capture) =>
    capture.componentCandidates.map((candidate) => candidate.representativePath),
  ),
);
captures.forEach((capture, captureIndex) => {
  for (const node of capture.nodes) {
    const values = byPath.get(node.path) || [];
    values[captureIndex] = node;
    byPath.set(node.path, values);
  }
});

const responsive = [];
for (const [nodePath, values] of byPath) {
  if (!componentPaths.has(nodePath)) continue;
  if (values.filter(Boolean).length < 2) continue;
  const [desktop, mobile] = values;
  if (
    desktop.nodeType !== 1 ||
    mobile.nodeType !== 1 ||
    (!desktop.visible && !mobile.visible)
  ) {
    continue;
  }
  const changes = [];
  if (desktop.visible !== mobile.visible) changes.push('visibility-breakpoint');
  if (desktop.style.display !== mobile.style.display) changes.push('display-breakpoint');
  if (desktop.style['flex-direction'] !== mobile.style['flex-direction']) changes.push('flex-direction-breakpoint');
  if (desktop.style['grid-template-columns'] !== mobile.style['grid-template-columns']) changes.push('grid-breakpoint');
  const widthDelta = desktop.rect.width - mobile.rect.width;
  const viewportDelta =
    captures[0].document.viewport.width - captures[1].document.viewport.width;
  if (
    desktop.visible &&
    mobile.visible &&
    desktop.rect.width >= 8 &&
    mobile.rect.width >= 8 &&
    Math.abs(widthDelta) < 2
  ) {
    changes.push('fixed-width');
  }
  else if (Math.abs(widthDelta - viewportDelta) < 4) changes.push('viewport-fluid');
  if (
    desktop.visible &&
    mobile.visible &&
    Math.abs(desktop.rect.x - mobile.rect.x) < 2
  ) {
    changes.push('left-anchored');
  }
  const desktopRight =
    captures[0].document.viewport.width - desktop.rect.right;
  const mobileRight =
    captures[1].document.viewport.width - mobile.rect.right;
  if (
    desktop.visible &&
    mobile.visible &&
    Math.abs(desktopRight - mobileRight) < 2
  ) {
    changes.push('right-anchored');
  }
  if (Math.abs(desktop.rect.y - mobile.rect.y) > 4) changes.push('vertical-reflow');
  if (changes.length) responsive.push({ path: nodePath, changes });
}

const output = {
  schemaVersion: 2,
  source: {
    requestedUrl,
    capturedUrl: captures[0]?.document.url,
    reusedAuthenticatedTarget: reuse,
    capturedAt: new Date().toISOString(),
  },
  viewports,
  captures,
  responsive,
  stylesheets: stylesheetManifest,
  scripts: scriptManifest,
  animationImplementations,
  animationLibrarySignals,
  dataAttributeImplementations,
  confidence: {
    authoredCss: stylesheetManifest.some((item) => item.file) ? 'high' : 'low',
    layout: captures.length >= 2 ? 'high' : 'medium',
    assets: 'medium',
    behavior: 'medium',
    notes: [
      'Listener presence and semantic attributes are captured; application state transitions inside minified handlers are not inferred.',
      'Closed shadow roots and cross-origin frame internals may be unavailable.',
      'Rects validate rendering; authored CSS and responsive deltas describe implementation constraints.',
    ],
  },
};

const isWithin = (pathValue, rootPath) =>
  pathValue === rootPath || pathValue?.startsWith(`${rootPath}>`);
const componentDir = path.join(outDir, 'components');
fs.mkdirSync(componentDir, { recursive: true });
const componentPackages = (captures[0]?.componentCandidates || []).map(
  (candidate, index) => {
    const filename = `component-${String(index + 1).padStart(3, '0')}.json`;
    const desktopNodes = captures[0].nodes.filter((node) =>
      isWithin(node.path, candidate.representativePath),
    );
    const root = desktopNodes.find(
      (node) => node.path === candidate.representativePath,
    );
    const heading = desktopNodes.find(
      (node) => /^h[1-6]$/.test(node.tag) && node.text?.trim(),
    )?.text;
    const classHint = String(root?.attrs?.class || '')
      .split(/\s+/)
      .find((value) => value && value.length < 80 && !/^[a-z0-9_-]{12,}$/i.test(value));
    const label =
      root?.ariaLabel ||
      heading?.trim() ||
      root?.attrs?.id ||
      root?.role ||
      (['header', 'nav', 'main', 'section', 'article', 'aside', 'footer', 'form', 'dialog'].includes(root?.tag)
        ? root.tag
        : classHint) ||
      root?.tag ||
      `component-${index + 1}`;
    const identity = {
      label: String(label).replace(/\s+/g, ' ').trim().slice(0, 120),
      tag: root?.tag,
      role: root?.role,
      ariaLabel: root?.ariaLabel,
      heading: heading?.trim().slice(0, 200),
      id: root?.attrs?.id,
      classHint,
    };
    const component = {
      id: `component-${String(index + 1).padStart(3, '0')}`,
      identity,
      candidate,
      captures: captures.map((capture) => ({
        viewport: capture.document.viewport,
        root: capture.nodes.find(
          (node) => node.path === candidate.representativePath,
        ),
        nodes: capture.nodes.filter((node) =>
          isWithin(node.path, candidate.representativePath),
        ),
        behaviors: capture.behaviors.filter((behavior) =>
          isWithin(behavior.path, candidate.representativePath),
        ),
        exactAssets: capture.exactAssets.filter((asset) =>
          isWithin(asset.path, candidate.representativePath),
        ),
        deepStyles: capture.deepStyles.filter((style) =>
          isWithin(style.path, candidate.representativePath),
        ),
        animationElements: (capture.animationElements || []).filter((element) =>
          isWithin(element.path, candidate.representativePath),
        ),
        lifecycleAnimationTracks: (
          capture.lifecycleAnimation?.tracks || []
        ).filter((track) =>
          isWithin(track.path, candidate.representativePath),
        ),
        horizontalTracks: (capture.horizontalTracks || []).filter((track) =>
          isWithin(track.path, candidate.representativePath),
        ),
      })),
      responsive: responsive.filter((rule) =>
        isWithin(rule.path, candidate.representativePath),
      ),
    };
    const componentAnimationTypes = [
      ...new Set(
        component.captures.flatMap((capture) =>
          capture.animationElements
            .map((element) => element.data?.['data-animation'])
            .filter(Boolean),
        ),
      ),
    ];
    component.animationImplementations = animationImplementations.filter(
      (implementation) =>
        componentAnimationTypes.includes(implementation.type),
    );
    fs.writeFileSync(
      path.join(componentDir, filename),
      JSON.stringify(component, null, 2),
    );
    return {
      id: component.id,
      file: `components/${filename}`,
      identity,
      path: candidate.representativePath,
      score: candidate.score,
      reasons: candidate.reasons,
      nodeCounts: component.captures.map((capture) => capture.nodes.length),
    };
  },
);
for (const component of componentPackages) {
  const parent = componentPackages
    .filter(
      (candidate) =>
        candidate.id !== component.id &&
        isWithin(component.path, candidate.path),
    )
    .sort((left, right) => right.path.length - left.path.length)[0];
  component.parentId = parent?.id;
}
for (const component of componentPackages) {
  component.childIds = componentPackages
    .filter((candidate) => candidate.parentId === component.id)
    .map((candidate) => candidate.id);
}
fs.writeFileSync(
  path.join(outDir, 'component-map.json'),
  JSON.stringify(componentPackages, null, 2),
);
output.componentPackages = componentPackages;

const validationErrors = [];
for (const [captureIndex, capture] of captures.entries()) {
  const paths = new Set(capture.nodes.map((node) => node.path));
  if (!capture.readiness?.ready) {
    validationErrors.push(`capture ${captureIndex}: application readiness timed out`);
  }
  if (
    !capture.initialDocument?.file ||
    !fs.existsSync(path.join(outDir, capture.initialDocument.file))
  ) {
    validationErrors.push(
      `capture ${captureIndex}: missing initial document artifact`,
    );
  }
  for (const candidate of capture.componentCandidates) {
    if (!paths.has(candidate.representativePath)) {
      validationErrors.push(
        `capture ${captureIndex}: missing component path ${candidate.representativePath}`,
      );
    }
  }
  for (const behavior of capture.behaviors) {
    if (!paths.has(behavior.path)) {
      validationErrors.push(
        `capture ${captureIndex}: missing behavior path ${behavior.path}`,
      );
    }
  }
  for (const asset of capture.exactAssets) {
    if (!paths.has(asset.path)) {
      validationErrors.push(
        `capture ${captureIndex}: missing asset path ${asset.path}`,
      );
    }
  }
  for (const element of capture.animationElements || []) {
    if (!paths.has(element.path)) {
      validationErrors.push(
        `capture ${captureIndex}: missing animation path ${element.path}`,
      );
    }
  }
  for (const track of capture.lifecycleAnimation?.tracks || []) {
    if (!track.samples?.length) {
      validationErrors.push(
        `capture ${captureIndex}: empty lifecycle track ${track.path}`,
      );
      continue;
    }
    if (
      track.samples.some(
        (sample, index) =>
          index > 0 && sample.time < track.samples[index - 1].time,
      )
    ) {
      validationErrors.push(
        `capture ${captureIndex}: non-monotonic lifecycle track ${track.path}`,
      );
    }
  }
  for (const track of capture.horizontalTracks || []) {
    if (!paths.has(track.path)) {
      validationErrors.push(
        `capture ${captureIndex}: missing horizontal track path ${track.path}`,
      );
    }
  }
}
for (const sheet of stylesheetManifest.filter((item) => item.file)) {
  if (!fs.existsSync(path.join(outDir, sheet.file))) {
    validationErrors.push(`missing stylesheet artifact ${sheet.file}`);
  }
}
for (const script of scriptManifest.filter((item) => item.file)) {
  if (!fs.existsSync(path.join(outDir, script.file))) {
    validationErrors.push(`missing script artifact ${script.file}`);
  }
}
for (const component of componentPackages) {
  if (!fs.existsSync(path.join(outDir, component.file))) {
    validationErrors.push(`missing component artifact ${component.file}`);
  }
}
if (!fs.existsSync(path.join(outDir, 'component-map.json'))) {
  validationErrors.push('missing component map artifact');
}

const allListeners = captures.flatMap((capture) =>
  [
    ...capture.behaviors.flatMap((behavior) => behavior.listeners),
    ...(capture.globalListeners || []),
  ],
);
const listenerEvidenceCount = allListeners.filter(
  (listener) => listener.sourceStatus === 'captured',
).length;
const opaqueListenerCount = allListeners.filter(
  (listener) => listener.sourceStatus === 'protocol-opaque',
).length;
const capturableScripts = scriptManifest.filter(
  (script) => script.status !== 'protocol-opaque',
);
const coverage = {
  listenerEvidence: {
    covered: listenerEvidenceCount,
    protocolOpaque: opaqueListenerCount,
    unavailable: allListeners.length - listenerEvidenceCount - opaqueListenerCount,
    total: allListeners.length,
    capturedRatio: allListeners.length
      ? listenerEvidenceCount / allListeners.length
      : 1,
    accountedRatio: allListeners.length
      ? (listenerEvidenceCount + opaqueListenerCount) / allListeners.length
      : 1,
  },
  matchedAuthoredRules: {
    covered: captures.reduce(
      (count, capture) =>
        count +
        capture.deepStyles.filter((style) => style.matchedRules.length).length,
      0,
    ),
    total: captures.reduce(
      (count, capture) => count + capture.deepStyles.length,
      0,
    ),
  },
  pseudoStateDeltas: {
    covered: captures.reduce(
      (count, capture) =>
        count +
        capture.deepStyles.filter(
          (style) => Object.keys(style.stateStyles).length,
        ).length,
      0,
    ),
    total: captures.reduce(
      (count, capture) => count + capture.deepStyles.length,
      0,
    ),
  },
  scriptBlobs: {
    covered: capturableScripts.filter((script) => script.file).length,
    protocolOpaque: scriptManifest.length - capturableScripts.length,
    total: capturableScripts.length,
  },
  stylesheetBlobs: {
    covered: stylesheetManifest.filter((sheet) => sheet.file).length,
    total: stylesheetManifest.length,
  },
  lifecycleAnimations: captures.map((capture) => {
    const animatedTracks = (capture.lifecycleAnimation?.tracks || []).filter(
      (track) => track.samples.length > 1,
    );
    const attributes = new Set(
      animatedTracks.flatMap((track) => Object.keys(track.data || {})),
    );
    const implementedAttributes = new Set(
      dataAttributeImplementations.map(
        (implementation) => implementation.attribute,
      ),
    );
    return {
      viewport: capture.document.viewport,
      durationMs: capture.lifecycleAnimation?.durationMs || 0,
      frameCount: capture.lifecycleAnimation?.frameCount || 0,
      trackCount: animatedTracks.length,
      sampleCount: animatedTracks.reduce(
        (count, track) => count + track.samples.length,
        0,
      ),
      browserDefinitionCount:
        capture.lifecycleAnimation?.animationDefinitions?.length || 0,
      dataAttributeImplementations: {
        covered: [...attributes].filter((attribute) =>
          implementedAttributes.has(attribute),
        ).length,
        total: attributes.size,
        ratio: attributes.size
          ? [...attributes].filter((attribute) =>
              implementedAttributes.has(attribute),
            ).length / attributes.size
          : 1,
      },
    };
  }),
  viewportStates: captures.map((capture) => {
    const visibleElementPaths = new Set(
      capture.nodes
        .filter((node) => node.nodeType === 1 && node.visible)
        .map((node) => node.path),
    );
    const observedPaths = new Set(
      capture.scrollStates.flatMap((state) =>
        state.checkpoints.flatMap((checkpoint) =>
          checkpoint.visibleElements.map((element) => element.path),
        ),
      ),
    );
    const animationPaths = new Set(
      (capture.animationElements || []).map((element) => element.path),
    );
    return {
      viewport: capture.document.viewport,
      visibleElements: {
        covered: [...visibleElementPaths].filter((pathValue) =>
          observedPaths.has(pathValue),
        ).length,
        total: visibleElementPaths.size,
      },
      animationElements: {
        covered: [...animationPaths].filter((pathValue) =>
          observedPaths.has(pathValue),
        ).length,
        total: animationPaths.size,
      },
    };
  }),
};
for (const metric of Object.values(coverage)) {
  if (
    !Array.isArray(metric) &&
    !('ratio' in metric) &&
    !('accountedRatio' in metric)
  ) {
    metric.ratio = metric.total ? metric.covered / metric.total : 1;
  }
}
for (const viewportCoverage of coverage.viewportStates) {
  for (const metric of [
    viewportCoverage.visibleElements,
    viewportCoverage.animationElements,
  ]) {
    metric.ratio = metric.total ? metric.covered / metric.total : 1;
  }
}
output.coverage = coverage;
output.validation = {
  passed: validationErrors.length === 0,
  errors: validationErrors,
};

fs.writeFileSync(path.join(outDir, 'spec.json'), JSON.stringify(output, null, 2));
fs.writeFileSync(
  path.join(outDir, 'summary.json'),
  JSON.stringify(
    {
      source: output.source,
      viewports,
      captures: captures.map((capture) => ({
        viewport: capture.document.viewport,
        document: capture.document,
        initialDocument: capture.initialDocument,
        nodeCount: capture.nodes.length,
        visibleNodeCount: capture.nodes.filter((node) => node.visible).length,
        componentCandidateCount: capture.componentCandidates.length,
        behaviorCount: capture.behaviors.length,
        globalListenerCount: (capture.globalListeners || []).length,
        resourceCount: capture.resources.length,
        fontCount: capture.fonts.length,
        animationCount: capture.animations.length,
        animationElementCount: (capture.animationElements || []).length,
        lifecycleAnimationDurationMs:
          capture.lifecycleAnimation?.durationMs || 0,
        lifecycleAnimationFrameCount:
          capture.lifecycleAnimation?.frameCount || 0,
        lifecycleAnimationTrackCount:
          capture.lifecycleAnimation?.tracks?.length || 0,
        lifecycleAnimationSampleCount: (
          capture.lifecycleAnimation?.tracks || []
        ).reduce((count, track) => count + track.samples.length, 0),
        lifecycleAnimationDefinitionCount:
          capture.lifecycleAnimation?.animationDefinitions?.length || 0,
        horizontalTrackCount: (capture.horizontalTracks || []).length,
        exactAssetCount: capture.exactAssets.length,
        deepStyleCount: capture.deepStyles.length,
        pseudoStateElementCount: capture.deepStyles.filter(
          (item) => Object.keys(item.stateStyles || {}).length,
        ).length,
        scrollContainerCount: capture.scrollStates.length,
        scrollCheckpointCount: capture.scrollStates.reduce(
          (count, state) => count + state.checkpoints.length,
          0,
        ),
        scrollAnimationSampleCount: capture.scrollStates.reduce(
          (count, state) =>
            count +
            state.checkpoints.reduce(
              (checkpointCount, checkpoint) =>
                checkpointCount + checkpoint.animations.length,
              0,
            ),
          0,
        ),
        timings: capture.timings,
        readiness: capture.readiness,
      })),
      responsiveRuleCount: responsive.length,
      matchedRulesMs,
      stylesheetCount: stylesheetManifest.length,
      listenerScriptCount: scriptManifest.length,
      animationImplementationCount: animationImplementations.length,
      animationLibraryScriptCount: animationLibrarySignals.length,
      dataAttributeImplementationCount: dataAttributeImplementations.length,
      componentPackageCount: componentPackages.length,
      mediaQueryCount: stylesheetManifest.reduce(
        (count, sheet) => count + (sheet.mediaQueries?.length || 0),
        0,
      ),
      keyframeCount: stylesheetManifest.reduce(
        (count, sheet) => count + (sheet.keyframes?.length || 0),
        0,
      ),
      pseudoSelectorCount: stylesheetManifest.reduce(
        (count, sheet) => count + (sheet.pseudoSelectors?.length || 0),
        0,
      ),
      customPropertyCount: stylesheetManifest.reduce(
        (count, sheet) => count + (sheet.customProperties?.length || 0),
        0,
      ),
      coverage,
      validation: output.validation,
      confidence: output.confidence,
    },
    null,
    2,
  ),
);

await cdp.send('Emulation.clearDeviceMetricsOverride');
if (reuse) await cdp.send('Page.reload', { ignoreCache: false });
if (created && browser && targetId) await browser.send('Target.closeTarget', { targetId });
cdp.close();
browser?.close();

console.log(fs.readFileSync(path.join(outDir, 'summary.json'), 'utf8'));
