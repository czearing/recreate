#!/usr/bin/env node

import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { createHash } from 'node:crypto';

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
const crawl = Boolean(args.crawl);
const maxRoutes = parseInt(String(args['max-routes'] || '30'), 10);
const profile = String(args.profile || 'implementation').toLowerCase();
if (!['implementation', 'full'].includes(profile)) {
  throw new Error('Pass --profile implementation or --profile full.');
}
const fullProfile = profile === 'full';
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
      if (this.ws.readyState !== WebSocket.OPEN) {
        return reject(new Error('CDP socket closed'));
      }
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
let cdp = await connect(page.webSocketDebuggerUrl);
const styleSheets = new Map();
const scripts = new Map();
let latestDocumentResponse;
let latestDocumentBody;
let mainFrameId;

const cdpDomains = [
  'Page.enable',
  'Runtime.enable',
  'DOM.enable',
  'CSS.enable',
  'DOMSnapshot.enable',
  'Network.enable',
  'Accessibility.enable',
  'Debugger.enable',
];

async function initializeCdp(client) {
  client.on('CSS.styleSheetAdded', ({ header }) => styleSheets.set(header.styleSheetId, header));
  client.on('CSS.styleSheetRemoved', ({ styleSheetId }) => styleSheets.delete(styleSheetId));
  client.on('Debugger.scriptParsed', (script) => scripts.set(script.scriptId, script));
  client.on('Network.responseReceived', (event) => {
    if (event.type === 'Document' && event.frameId === mainFrameId) {
      latestDocumentResponse = event;
    }
  });
  client.on('Network.loadingFinished', async ({ requestId }) => {
    if (requestId !== latestDocumentResponse?.requestId) return;
    try {
      latestDocumentBody = await client.send('Network.getResponseBody', { requestId });
    } catch (error) {
      latestDocumentBody = { error: String(error) };
    }
  });
  for (const domain of cdpDomains) await client.send(domain);
  mainFrameId = (await client.send('Page.getFrameTree')).frameTree.frame.id;
}

await initializeCdp(cdp);

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
  let lastNodeCount = 0;
  let stableCount = 0;
  // Slow authenticated prototypes can keep a visual splash after DOM readiness.
  for (let attempt = 0; attempt < 180; attempt++) {
    try {
      state = (
        await cdp.send('Runtime.evaluate', {
          expression: `({
            readyState: document.readyState,
            fonts: document.fonts.status,
            isLoading: document.documentElement.classList.contains('is-loading'),
            isLoaded: document.documentElement.classList.contains('is-loaded'),
            hasLenisWrapper: Boolean(window.lenis),
            hasLenisScroll: Boolean(window.lenis?.scroll),
            hasContent: Boolean(
              document.body &&
              (document.body.children.length || (document.body.textContent || '').trim())
            ),
            hasBlockingVisual: Array.from(new Set([
              ...Array.from(document.body?.children || []),
              ...document.querySelectorAll([
                'img',
                '[aria-busy="true"]',
                '[role="progressbar"]',
                '[id*="load" i]',
                '[class*="load" i]',
                '[id*="splash" i]',
                '[class*="splash" i]',
                '[id*="intro" i]',
                '[class*="intro" i]',
                '[id*="boot" i]',
                '[class*="boot" i]'
              ].join(','))
            ])).some(element => {
              const rect = element.getBoundingClientRect();
              const style = getComputedStyle(element);
              const identity = [
                element instanceof HTMLImageElement ? element.currentSrc : '',
                element.getAttribute('alt'),
                element.getAttribute('aria-label'),
                element.getAttribute('aria-busy'),
                element.getAttribute('role'),
                element.id,
                element.className,
                element.textContent?.trim().slice(0, 100),
              ].join(' ');
              const coversViewport =
                rect.width * rect.height > innerWidth * innerHeight * 0.5;
              const parentRect = element.parentElement?.getBoundingClientRect();
              const largeSplashImage =
                element instanceof HTMLImageElement &&
                rect.width * rect.height > innerWidth * innerHeight * 0.1 &&
                parentRect &&
                parentRect.width * parentRect.height >
                  innerWidth * innerHeight * 0.75;
              const visuallyPresent =
                style.display !== 'none' &&
                style.visibility !== 'hidden' &&
                Number(style.opacity || 1) > 0;
              return (
                /(?:load|splash|intro|boot)/i.test(identity) &&
                visuallyPresent &&
                (
                  coversViewport ||
                  largeSplashImage ||
                  element.getAttribute('aria-busy') === 'true' ||
                  element.getAttribute('role') === 'progressbar'
                )
              );
            }),
            nodeCount: document.querySelectorAll('*').length
          })`,
          returnByValue: true,
        })
      ).result.value;

      const baseReady =
        state.readyState === 'complete' &&
        state.fonts === 'loaded' &&
        (!state.isLoading || state.isLoaded) &&
        (!state.hasLenisWrapper || state.hasLenisScroll) &&
        !state.hasBlockingVisual;

      if (baseReady) {
        const n = state.nodeCount ?? 0;
        if (state.hasContent && n > 0 && n === lastNodeCount) {
          stableCount++;
          // Require 3 stable polls (~750ms) before declaring ready
          if (stableCount >= 3) {
            return {
              ready: true,
              waitMs: Date.now() - startedAt,
              state,
            };
          }
        } else {
          stableCount = 0;
          lastNodeCount = n;
        }
      } else {
        stableCount = 0;
        lastNodeCount = 0;
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
let navigationDoneByMultiPage = false;
const multiPageStates = [];
let homePageState;

function absolutizeCssUrls(cssText, sourceUrl) {
  if (!sourceUrl) return cssText;
  return cssText.replace(
    /url\(\s*(['"]?)(?!data:|blob:|https?:|\/\/|#)([^'")]+)\1\s*\)/gi,
    (match, quote, value) => {
      try {
        return `url("${new URL(value.trim(), sourceUrl).href}")`;
      } catch {
        return match;
      }
    },
  );
}

async function capturePageSnapshot(index, slug = String(index).padStart(3, '0')) {
  await cdp.send('Runtime.evaluate', {
    expression: `Promise.race([
      new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))),
      new Promise(resolve => setTimeout(resolve, 100))
    ])`,
    awaitPromise: true,
    returnByValue: true,
  }).catch(() => {});
  const pageDir = path.join(outDir, 'pages');
  fs.mkdirSync(pageDir, { recursive: true });
  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: 1440, height: 900, deviceScaleFactor: 1, mobile: false,
  }).catch(() => {});

  let pageData = {};
  let screenshot;
  const [snapshotResult, screenshotResult] = await Promise.allSettled([
    cdp.send('Runtime.evaluate', {
        expression: `JSON.stringify({
            url: location.href,
            title: document.title,
            nodeCount: document.querySelectorAll('*').length,
            text: (document.body || document.documentElement).innerText.substring(0, 3000),
            bodyHeight: document.body ? document.body.scrollHeight : 0,
            html: '<!DOCTYPE html>\\n' + document.documentElement.outerHTML,
            stylesheets: Array.from(document.styleSheets).map(sheet => {
              try {
                return {
                  href: sheet.href || location.href,
                  text: Array.from(sheet.cssRules || [], rule => rule.cssText).join('\\n')
                };
              } catch {
                return null;
              }
            }).filter(Boolean)
          })`,
        returnByValue: true,
      }),
    cdp.send('Page.captureScreenshot', { format: 'png' }),
  ]);
  if (snapshotResult.status === 'fulfilled') {
    pageData = JSON.parse(snapshotResult.value.result?.value || '{}');
  }
  if (screenshotResult.status === 'fulfilled') {
    screenshot = screenshotResult.value.data;
    fs.writeFileSync(path.join(pageDir, `${slug}.png`), Buffer.from(screenshot, 'base64'));
  }

  let htmlFile;
  if (pageData.html) {
    fs.writeFileSync(path.join(pageDir, `${slug}.html`), pageData.html);
    htmlFile = `pages/${slug}.html`;
  }

  let stylesheetFile;
  const stylesheetText = (pageData.stylesheets || [])
    .map((sheet) => absolutizeCssUrls(sheet.text || '', sheet.href))
    .filter(Boolean)
    .join('\n');
  if (stylesheetText) {
    fs.writeFileSync(path.join(pageDir, `${slug}.css`), stylesheetText);
    stylesheetFile = `pages/${slug}.css`;
  }

  return {
    index,
    type: 'route',
    url: pageData.url || '',
    title: pageData.title || '',
    nodeCount: pageData.nodeCount || 0,
    text: pageData.text || '',
    bodyHeight: pageData.bodyHeight || 0,
    html: htmlFile,
    stylesheet: stylesheetFile,
    screenshot: screenshot ? `pages/${slug}.png` : undefined,
    capturedAt: new Date().toISOString(),
  };
}

async function navigateAndCaptureAllPages(targetUrl) {
  console.error('phase: multi-page navigate');

  // Intercept pushState/replaceState so URL changes inside SPAs are visible via polling
  await cdp.send('Page.addScriptToEvaluateOnNewDocument', {
    source: `(function(){
      const orig = history.pushState.bind(history);
      history.pushState = function(){ orig.apply(this, arguments); window.__siteSpecUrlChanged = location.href; };
      const origR = history.replaceState.bind(history);
      history.replaceState = function(){ origR.apply(this, arguments); window.__siteSpecUrlChanged = location.href; };
    })();`
  });

  await cdp.send('Page.navigate', { url: targetUrl });

  let lastCapturedUrl = '';
  let lastCapturedNodeCount = 0;
  let lastUrl = '';
  let lastNodeCount = 0;
  let sameUrlPolls = 0;
  const deadline = Date.now() + 40000;
  let targetHost;
  try { targetHost = new URL(targetUrl).hostname; } catch (_) {}

  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 200));

    let currentUrl = '', currentNodeCount = 0, readyState = '';
    try {
      const r = await cdp.send('Runtime.evaluate', {
        expression: `JSON.stringify({u:location.href,n:document.querySelectorAll('*').length,rs:document.readyState})`,
        returnByValue: true,
      });
      const d = JSON.parse(r.result?.value || '{}');
      currentUrl = d.u || '';
      currentNodeCount = d.n || 0;
      readyState = d.rs || '';
    } catch (_) { sameUrlPolls = 0; continue; }

    const urlChanged = currentUrl !== lastUrl;
    if (urlChanged) {
      sameUrlPolls = 0;
      lastUrl = currentUrl;
      lastNodeCount = currentNodeCount;
      continue;
    }

    if (readyState !== 'complete') {
      sameUrlPolls = 0;
      lastNodeCount = currentNodeCount;
      continue;
    }

    const nodeCountDelta = Math.abs(currentNodeCount - lastNodeCount);
    if (nodeCountDelta > 10) {
      sameUrlPolls = 0;
      lastNodeCount = currentNodeCount;
      continue;
    }

    sameUrlPolls++;

    // Capture after 2 stable polls (~400ms) if this state looks new
    const nodeCountJump = Math.abs(currentNodeCount - lastCapturedNodeCount) > 30;
    const newUrl = currentUrl !== lastCapturedUrl;

    if (sameUrlPolls >= 2 && (newUrl || nodeCountJump)) {
      try {
        const currentHost = new URL(currentUrl).hostname;
        if (currentHost === targetHost && currentNodeCount > 0) break;
      } catch (_) {}

      lastCapturedUrl = currentUrl;
      lastCapturedNodeCount = currentNodeCount;
      console.error(`phase: capture page state ${multiPageStates.length} url=${currentUrl} nodes=${currentNodeCount}`);
      const state = await capturePageSnapshot(multiPageStates.length);
      multiPageStates.push(state);
      sameUrlPolls = 0;

    }
  }

  navigationDoneByMultiPage = true;
  console.error(`phase: multi-page done — ${multiPageStates.length} page states captured`);
}

async function crawlRoutes(baseUrl, maxRoutes = 30) {
  console.error('phase: route crawl start');

  const currentUrl = (await cdp.send('Runtime.evaluate', {
    expression: 'location.href', returnByValue: true,
  }).catch(() => ({ result: { value: '' } }))).result.value;
  if (baseUrl && currentUrl !== baseUrl) {
    await cdp.send('Page.navigate', { url: baseUrl });
    await waitForApplicationReady();
  }

  // Inject pushState intercept on the live page so clicks are trackable
  await cdp.send('Runtime.evaluate', {
    expression: `(function(){
      if (window.__siteSpecCrawlReady) return;
      window.__siteSpecCrawlReady = true;
      window.__siteSpecLastPush = location.href;
      const orig = history.pushState.bind(history);
      history.pushState = function(s,t,u){ orig(s,t,u); window.__siteSpecLastPush = location.href; };
      const origR = history.replaceState.bind(history);
      history.replaceState = function(s,t,u){ origR(s,t,u); window.__siteSpecLastPush = location.href; };
    })()`,
    returnByValue: true,
  }).catch(() => {});

  const visitedUrls = new Set(multiPageStates.map((p) => p.url));
  const homeUrl = (await cdp.send('Runtime.evaluate', {
    expression: 'location.href', returnByValue: true,
  })).result.value;
  visitedUrls.add(homeUrl);

  const initialNodeCount = (await cdp.send('Runtime.evaluate', {
    expression: 'document.querySelectorAll("*").length', returnByValue: true,
  }).catch(() => ({ result: { value: 0 } }))).result.value || 0;
  console.error(`phase: route crawl initial node count: ${initialNodeCount}`);

  let baseHost;
  try { baseHost = new URL(baseUrl).hostname; } catch (_) {}

  // Find all candidate clickable elements that might trigger route changes
  const candidatesResult = await cdp.send('Runtime.evaluate', {
    expression: `JSON.stringify(
      Array.from(document.querySelectorAll('a[href], [role="link"], [role="button"], button, [data-href], [data-url], [tabindex]:not([tabindex="-1"])'))
        .filter(el => {
          const rect = el.getBoundingClientRect();
          // Skip header/toolbar elements (top 80px) — they toggle panels, not navigate routes
          const scrollTop = window.scrollY || document.documentElement.scrollTop;
          return rect.width > 4 && rect.height > 4 && (rect.top + scrollTop) > 80;
        })
        .map((el, i) => ({
          i,
          tag: el.tagName,
          href: el.getAttribute('href') || el.getAttribute('data-href') || el.getAttribute('data-url') || '',
          text: (el.getAttribute('aria-label') || el.innerText || '').substring(0, 500).trim(),
          role: el.getAttribute('role') || '',
          testId: el.getAttribute('data-testid') || '',
          y: Math.round(el.getBoundingClientRect().top),
          snapshotPath: (() => {
            const parts = [];
            let node = el;
            while (node && node.nodeType === Node.ELEMENT_NODE) {
              const tag = node.tagName.toLowerCase();
              const siblings = Array.from(node.parentElement?.children || [])
                .filter((sibling) => sibling.tagName === node.tagName);
              const position = siblings.indexOf(node);
              parts.unshift(tag + ':nth-of-type(' + (position >= 0 ? position + 1 : 1) + ')');
              node = node.parentElement;
            }
            return 'doc(0)>' + parts.join('>');
          })(),
          path: (() => {
            let p = [], n = el; 
            while(n && n !== document.body){ p.unshift(Array.from(n.parentElement?.children||[]).indexOf(n)); n=n.parentElement; }
            return p.join('>');
          })()
        }))
        .filter(e => e.text || e.href)
        .slice(0, 60)
    )`,
    returnByValue: true,
  });

  let candidates = [];
  try { candidates = JSON.parse(candidatesResult.result?.value || '[]'); } catch (_) {}
  const candidatePriority = (candidate) => {
    if (candidate.href) return 100;
    if (candidate.testId === 'notebook-card') return 90;
    if (candidate.role === 'link' || candidate.tag === 'A') return 80;
    if (candidate.role === 'button') return 50;
    return 10;
  };
  candidates.sort((left, right) => candidatePriority(right) - candidatePriority(left));
  console.error(`phase: route crawl found ${candidates.length} candidates`);
  const triggerElementFor = (candidate) => ({
    path: candidate.snapshotPath,
    label: candidate.text,
    tag: candidate.tag.toLowerCase(),
    role: candidate.role || undefined,
    testId: candidate.testId || undefined,
  });

  for (const candidate of candidates) {
    if (multiPageStates.length >= maxRoutes) break;

    // Always navigate back to home before each candidate to ensure clean state
    const currentUrl = (await cdp.send('Runtime.evaluate', {
      expression: 'location.href', returnByValue: true,
    }).catch(() => ({ result: { value: '' } }))).result.value;

    if (currentUrl !== homeUrl) {
      await cdp.send('Page.navigate', { url: homeUrl }).catch(() => {});
    }

    // Wait for home page to stabilize
    let homeN = 0, homeStab = 0;
    const backDeadline = Date.now() + 10000;
    while (Date.now() < backDeadline) {
      await new Promise((r) => setTimeout(r, 200));
      try {
        const r2 = await cdp.send('Runtime.evaluate', {
          expression: 'JSON.stringify({u:location.href,n:document.querySelectorAll("*").length})',
          returnByValue: true,
        });
        const d2 = JSON.parse(r2.result?.value || '{}');
        if (d2.u === homeUrl && Math.abs(d2.n - homeN) < 5 && d2.n > 0) {
          homeStab++;
          if (homeStab >= 2) break;
        } else { homeStab = 0; }
        homeN = d2.n;
      } catch (_) {}
    }

    // If node count is far from expected, an overlay is open — dismiss and re-navigate
    if (Math.abs(homeN - initialNodeCount) > initialNodeCount * 0.3) {
      for (const evType of ['keyDown', 'keyUp']) {
        await cdp.send('Input.dispatchKeyEvent', {
          type: evType, key: 'Escape', code: 'Escape', keyCode: 27, windowsVirtualKeyCode: 27,
        }).catch(() => {});
      }
      await new Promise((r) => setTimeout(r, 400));
      // Click body to dismiss any overlay
      await cdp.send('Runtime.evaluate', {
        expression: 'document.body.click()', returnByValue: true,
      }).catch(() => {});
      await new Promise((r) => setTimeout(r, 500));
      // Hard re-navigate if still off
      const reCheckN = (await cdp.send('Runtime.evaluate', {
        expression: 'document.querySelectorAll("*").length', returnByValue: true,
      }).catch(() => ({ result: { value: homeN } }))).result.value;
      if (Math.abs(reCheckN - initialNodeCount) > initialNodeCount * 0.3) {
        await cdp.send('Page.navigate', { url: homeUrl }).catch(() => {});
        await waitForApplicationReady();
      }
    }

    // Dismiss any open panels/menus left by previous click
    for (const evType of ['keyDown', 'keyUp']) {
      await cdp.send('Input.dispatchKeyEvent', {
        type: evType, key: 'Escape', code: 'Escape', keyCode: 27, windowsVirtualKeyCode: 27,
      }).catch(() => {});
    }
    await new Promise((r) => setTimeout(r, 100));

    // Log node count to verify page is fully rendered
    const preClickState = (await cdp.send('Runtime.evaluate', {
      expression: 'JSON.stringify({n:document.querySelectorAll("*").length,u:location.href})',
      returnByValue: true,
    }).catch(() => ({ result: { value: '{}' } }))).result.value;
    const { n: preClickN } = JSON.parse(preClickState);
    console.error(`phase: route crawl pre-click url: ${currentUrl.split('/').slice(-2).join('/')} nodes:${preClickN}`);

    // If it has an explicit href pointing to same origin, use it directly
    let targetRoute = '';
    if (candidate.href && !candidate.href.startsWith('http') && !candidate.href.startsWith('./') &&
        !candidate.href.match(/\.(js|css|svg|png|jpg|woff|ico)$/)) {
      targetRoute = candidate.href;
    } else if (candidate.href && candidate.href.startsWith('http')) {
      try {
        const u = new URL(candidate.href);
        if (u.hostname === baseHost) targetRoute = u.pathname + u.search;
      } catch (_) {}
    }

    const beforeUrl = (await cdp.send('Runtime.evaluate', {
      expression: 'location.href', returnByValue: true,
    }).catch(() => ({ result: { value: '' } }))).result.value;

    if (targetRoute) {
      // Direct navigation
      const fullUrl = targetRoute.startsWith('/') ? `${new URL(baseUrl).origin}${targetRoute}` : targetRoute;
      if (visitedUrls.has(fullUrl)) continue;
    }

    // Reset intercept and click the element
    await cdp.send('Runtime.evaluate', {
      expression: `window.__siteSpecLastPush = location.href`,
      returnByValue: true,
    }).catch(() => {});

    const clicked = await cdp.send('Runtime.evaluate', {
      expression: `(() => {
        const all = document.querySelectorAll('a[href], [role="link"], [role="button"], button, [data-href], [data-url], [tabindex]:not([tabindex="-1"])');
        // Search all rendered elements (not just in-viewport) — scroll into view before clicking
        const rendered = Array.from(all).filter(el => { const r = el.getBoundingClientRect(); return r.width > 4 && r.height > 4; });
        const path = ${JSON.stringify(candidate.path)}.split('>').map(Number);
        let el = path.reduce((node, index) => node?.children?.[index], document.body);
        const searchText = ${JSON.stringify(candidate.text.substring(0, 20))};
        const matchesCandidate = e =>
          rendered.includes(e) &&
          e.tagName === ${JSON.stringify(candidate.tag)} &&
          (e.getAttribute('role') || '') === ${JSON.stringify(candidate.role)} &&
          (e.getAttribute('data-testid') || '') === ${JSON.stringify(candidate.testId)} &&
          (e.getAttribute('aria-label') || e.innerText || '').trim().startsWith(searchText);
        if (!matchesCandidate(el)) {
          el = rendered.find(matchesCandidate);
        }
        if (!el) return 'not-found:' + searchText;
        el.scrollIntoView({ block: 'center', behavior: 'instant' });
        el.click();
        return 'clicked:' + (el.getAttribute('aria-label') || el.innerText || '').trim().substring(0, 30);
      })()`,
      returnByValue: true,
    }).catch(() => ({ result: { value: 'error' } }));

    const clickedVal = clicked.result?.value || 'error';
    console.error(`phase: route crawl try "${candidate.text.substring(0,30)}" → ${clickedVal}`);

    if (!clickedVal.startsWith('clicked')) continue;

    // Poll for URL change, modal, or DOM shift — up to 3 seconds
    let afterUrl = beforeUrl;
    let nodeCount = 0;
    let modalAppeared = false;
    const clickDeadline = Date.now() + 3000;
    while (Date.now() < clickDeadline) {
      await new Promise((r) => setTimeout(r, 300));
      try {
        const r = await cdp.send('Runtime.evaluate', {
          expression: `JSON.stringify({
            u: location.href,
            n: document.querySelectorAll('*').length,
            modal: !!document.querySelector('dialog[open],[role="dialog"],[role="alertdialog"],[aria-modal="true"],.modal,[data-modal]'),
            modalText: (document.querySelector('dialog[open],[role="dialog"],[role="alertdialog"],[aria-modal="true"]')?.innerText||'').substring(0,200)
          })`,
          returnByValue: true,
        });
        const d = JSON.parse(r.result?.value || '{}');
        afterUrl = d.u || beforeUrl;
        nodeCount = d.n || nodeCount;
        modalAppeared = d.modal && d.modalText?.trim().length > 0;
        // Stop on URL change, modal, or significant DOM growth (panel/drawer opened)
        if (afterUrl !== beforeUrl || modalAppeared || (nodeCount > preClickN * 1.05 && nodeCount - preClickN > 15)) break;
      } catch (_) { break; }
    }

    const urlChanged = afterUrl !== beforeUrl;
    const isInternal = (() => {
      try { return new URL(afterUrl).hostname === baseHost; } catch (_) { return false; }
    })();
    const panelOpened = !urlChanged && !modalAppeared && nodeCount > preClickN * 1.05 && nodeCount - preClickN > 15;

    // Capture modal state if one appeared (no URL change needed)
    if (!urlChanged && modalAppeared) {
      const modalKey = `modal:${candidate.text.substring(0, 40)}`;
      if (!visitedUrls.has(modalKey)) {
        visitedUrls.add(modalKey);
        // Let it fully render
        await new Promise((r) => setTimeout(r, 400));
        console.error(`phase: route crawl capture ${multiPageStates.length} modal triggered by "${candidate.text}"`);
        const state = await capturePageSnapshot(multiPageStates.length);
        // Tag it as a modal state
        state.type = 'modal';
        state.trigger = candidate.text;
        state.triggerElement = triggerElementFor(candidate);
        multiPageStates.push(state);
        // Dismiss modal
        await cdp.send('Runtime.evaluate', {
          expression: `document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}))`,
          returnByValue: true,
        }).catch(() => {});
        await new Promise((r) => setTimeout(r, 250));
      }
      continue;
    }

    // Capture panel/drawer state if DOM grew significantly without URL change
    if (panelOpened) {
      const panelKey = `panel:${candidate.path}`;
      if (!visitedUrls.has(panelKey)) {
        visitedUrls.add(panelKey);
        await new Promise((r) => setTimeout(r, 400));
        console.error('phase: route crawl capture ' + multiPageStates.length + ' panel by "' + candidate.text + '" nodes:' + preClickN + '->' + nodeCount);
        const state = await capturePageSnapshot(multiPageStates.length);
        state.type = 'panel';
        state.trigger = candidate.text;
        state.triggerElement = triggerElementFor(candidate);
        multiPageStates.push(state);
        // Return to home to reset state
        await cdp.send('Page.navigate', { url: homeUrl }).catch(() => {});
        await waitForApplicationReady();
      }
      continue;
    }

    if (!urlChanged || !isInternal || visitedUrls.has(afterUrl)) {
      // Navigate back if we went somewhere unexpected
      if (urlChanged && afterUrl !== homeUrl) {
        await cdp.send('Page.navigate', { url: homeUrl }).catch(() => {});
        await new Promise((r) => setTimeout(r, 1500));
      }
      continue;
    }

    visitedUrls.add(afterUrl);

    // Wait for this page to stabilize
    let stable = 0, lastN = 0;
    const deadline = Date.now() + 8000;
    while (Date.now() < deadline) {
      await new Promise((r) => setTimeout(r, 300));
      try {
        const r = await cdp.send('Runtime.evaluate', {
          expression: 'JSON.stringify({n:document.querySelectorAll("*").length,rs:document.readyState})',
          returnByValue: true,
        });
        const d = JSON.parse(r.result?.value || '{}');
        if (d.rs === 'complete' && Math.abs(d.n - lastN) < 5) {
          stable++;
          if (stable >= 3) { nodeCount = d.n; break; }
        } else { stable = 0; }
        lastN = d.n;
      } catch (_) { break; }
    }

    console.error(`phase: route crawl capture ${multiPageStates.length} url=${afterUrl} nodes=${nodeCount} text="${candidate.text}"`);
    const state = await capturePageSnapshot(multiPageStates.length);
    state.trigger = candidate.text;
    state.triggerElement = triggerElementFor(candidate);
    multiPageStates.push(state);

    // Navigate back to home
    await cdp.send('Page.navigate', { url: homeUrl }).catch(() => {});
  }

  console.error(`phase: route crawl done — ${multiPageStates.length} total page states`);
}

async function extractViewport(viewport, captureIndex) {
  const timings = {};
  let phaseStart = Date.now();
  console.error('phase: metrics');
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
  if (created && captureIndex === 0 && !navigationDoneByMultiPage) {
    await cdp.send('Page.navigate', { url });
  } else {
    await cdp.send('Page.reload', { ignoreCache: false });
  }
  console.error('phase: wait ready');
  const readiness = await waitForApplicationReady();
  console.error('phase: ready done', JSON.stringify(readiness));
  let initialDocument = {
    url: latestDocumentResponse?.response?.url,
    status: latestDocumentResponse?.response?.status,
    mimeType: latestDocumentResponse?.response?.mimeType,
  };
  try {
    const documentDir = path.join(outDir, 'documents');
    const filename = `${viewport.width}x${viewport.height}.html`;
    fs.mkdirSync(documentDir, { recursive: true });
    let body;
    if (latestDocumentBody && !latestDocumentBody.error) {
      body = latestDocumentBody.base64Encoded
        ? Buffer.from(latestDocumentBody.body, 'base64')
        : Buffer.from(latestDocumentBody.body);
    } else {
      const outerHtml = (
        await cdp.send('Runtime.evaluate', {
          expression: "(() => '<!DOCTYPE html>\n' + document.documentElement.outerHTML)()",
          returnByValue: true,
        })
      ).result.value;
      body = Buffer.from(String(outerHtml || ''), 'utf8');
      if (latestDocumentBody?.error) initialDocument.error = latestDocumentBody.error;
    }
    fs.writeFileSync(path.join(documentDir, filename), body);
    initialDocument = {
      ...initialDocument,
      file: `documents/${filename}`,
      length: body.length,
    };
  } catch (error) {
    initialDocument.error = String(error);
  }
  await cdp.send('Runtime.evaluate', {
    expression: `(() => {
      window.lenis?.refresh?.();
      window.lenis?.scroll?.resize?.();
      window.dispatchEvent(new Event('resize'));
    })()`,
  });
  await new Promise((resolve) => setTimeout(resolve, 100));
  console.error('phase: settle page');
  await settlePage();
  console.error('phase: settled');

  // Capture CSSOM stylesheet texts early while session is fresh, before heavy phases
  const cssomTexts = {};
  try {
    const storeResult = await cdp.send('Runtime.evaluate', {
      expression: `(function() {
        const out = {};
        let injIdx = 0;
        for (const sheet of [...document.styleSheets, ...(document.adoptedStyleSheets||[])]) {
          try {
            let text = '';
            for (const rule of (sheet.cssRules||[])) { text += rule.cssText + '\\n'; }
            const key = sheet.href || ('__injected_' + injIdx++);
            if (text.trim()) out[key] = text;
          } catch(e) { injIdx++; }
        }
        window.__spec_css = out;
        return JSON.stringify(Object.keys(out));
      })()`,
      returnByValue: true,
    });
    const keys = JSON.parse(storeResult.result?.value || '[]');
    for (const key of keys) {
      try {
        const r = await cdp.send('Runtime.evaluate', {
          expression: `window.__spec_css[${JSON.stringify(key)}]`,
          returnByValue: true,
        });
        if (r.result?.value) cssomTexts[key] = r.result.value;
      } catch (_) {}
    }
    await cdp.send('Runtime.evaluate', { expression: 'delete window.__spec_css', returnByValue: true }).catch(() => {});
  } catch (_) {}

  console.error('phase: lifecycle export');
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
  console.error('phase: scroll candidates');
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
  console.error('phase: snapshot');
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
  console.error('phase: supplement');
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
  console.error('phase: descriptors');
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
  console.error('phase: deep styles skipped');
  extracted.deepStyles = [];
  timings.deepStylesMs = Date.now() - phaseStart;

  extracted.cssomTexts = cssomTexts;

  phaseStart = Date.now();
  extracted.accessibility = { skipped: true };
  timings.accessibilityMs = Date.now() - phaseStart;
  timings.totalMs = Object.values(timings).reduce(
    (total, duration) => total + duration,
    0,
  );
  extracted.timings = timings;
  console.error('phase: extracted done');
  return extracted;
}

// Extract session cookies scoped to the target domain (avoids HTTP 431 on servers with header size limits)
let authCookieHeader = '';
try {
  const { cookies } = await cdp.send('Network.getAllCookies');
  if (cookies && cookies.length) {
    const targetHost = new URL(requestedUrl).hostname;
    const relevant = cookies.filter(c => targetHost === c.domain || targetHost.endsWith('.' + c.domain) || c.domain.endsWith('.' + targetHost));
    authCookieHeader = (relevant.length ? relevant : cookies).map(c => `${c.name}=${c.value}`).join('; ');
  }
} catch (_) {}

async function inlineSnapshotImages(snapshots) {
  const files = snapshots
    .filter((snapshot) => snapshot?.html)
    .map((snapshot) => path.join(outDir, snapshot.html))
    .filter((file) => fs.existsSync(file));
  const htmlByFile = new Map(
    files.map((file) => [file, fs.readFileSync(file, 'utf8')]),
  );
  const imageUrls = [
    ...new Set(
      [...htmlByFile.values()].flatMap((html) =>
        [...html.matchAll(/<img\b[^>]*\bsrc=(["'])(https?:\/\/[^"']+)\1/gi)]
          .map((match) => match[2]),
      ),
    ),
  ];
  const replacements = new Map();
  const assetDir = path.join(outDir, 'snapshot-assets');
  const targetHost = new URL(requestedUrl).hostname;
  let nextIndex = 0;
  const workers = Array.from(
    { length: Math.min(6, imageUrls.length) },
    async () => {
      while (nextIndex < imageUrls.length) {
        const imageUrl = imageUrls[nextIndex++];
        try {
          const imageHost = new URL(imageUrl).hostname;
          const sameSite =
            imageHost === targetHost ||
            imageHost.endsWith(`.${targetHost}`) ||
            targetHost.endsWith(`.${imageHost}`);
          const response = await fetch(imageUrl, {
            headers:
              sameSite && authCookieHeader
                ? { cookie: authCookieHeader }
                : {},
            signal: AbortSignal.timeout(5000),
          });
          if (!response.ok) continue;
          const contentType = response.headers.get('content-type') || '';
          if (!contentType.startsWith('image/')) continue;
          const bytes = Buffer.from(await response.arrayBuffer());
          if (bytes.length > 5 * 1024 * 1024) continue;
          fs.mkdirSync(assetDir, { recursive: true });
          const extensionByType = {
            'image/avif': '.avif',
            'image/gif': '.gif',
            'image/jpeg': '.jpg',
            'image/png': '.png',
            'image/svg+xml': '.svg',
            'image/webp': '.webp',
          };
          const extension = extensionByType[contentType.split(';')[0]] || '.img';
          const filename = `${createHash('sha256').update(bytes).digest('hex').slice(0, 20)}${extension}`;
          const assetPath = path.join(assetDir, filename);
          if (!fs.existsSync(assetPath)) fs.writeFileSync(assetPath, bytes);
          replacements.set(
            imageUrl,
            fullProfile
              ? `data:${contentType};base64,${bytes.toString('base64')}`
              : `/snapshot-assets/${filename}`,
          );
        } catch {}
      }
    },
  );
  await Promise.all(workers);
  for (const [file, originalHtml] of htmlByFile) {
    let html = originalHtml;
    for (const [imageUrl, dataUrl] of replacements) {
      html = html.split(imageUrl).join(dataUrl);
    }
    fs.writeFileSync(file, html);
  }
  return replacements.size;
}

const captures = [];

if (created) {
  await navigateAndCaptureAllPages(url);
}
await waitForApplicationReady();
homePageState = await capturePageSnapshot(-1, 'home');
homePageState.type = 'home';
if (crawl) {
  await crawlRoutes(requestedUrl, maxRoutes);
}
console.error(
  `phase: localized ${await inlineSnapshotImages([
    homePageState,
    ...multiPageStates,
  ])} snapshot images`,
);

for (let captureIndex = 0; captureIndex < viewports.length; captureIndex++) {
  if (captureIndex > 0) {
    cdp.close();
    const refreshedPage = (await getJson('/json/list')).find(
      (item) => item.id === page.id,
    );
    if (!refreshedPage) {
      throw new Error(`Browser target disappeared before viewport ${captureIndex + 1}.`);
    }
    cdp = await connect(refreshedPage.webSocketDebuggerUrl);
    styleSheets.clear();
    scripts.clear();
    latestDocumentResponse = undefined;
    latestDocumentBody = undefined;
    await initializeCdp(cdp);
  }
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
    let text = '';
    // 1. Try CDP in-browser text (works for auth-gated and CORS-restricted sheets)
    try {
      const result = await cdp.send('CSS.getStyleSheetText', { styleSheetId });
      text = result.text || '';
    } catch (_cdpErr) {}

    // 2. For injected sheets (CSS-in-JS) that have no text from CDP, use the in-page CSSOM serialization
    if (!text && !header.sourceURL) {
      const allCssomTexts = captures.flatMap(c => Object.values(c.cssomTexts || {}));
      // Use __injected_N keyed entry from the capture that corresponds to this sheet
      const injIdx = [...styleSheets.keys()].filter(id => !styleSheets.get(id).sourceURL).indexOf(styleSheetId);
      text = captures[0]?.cssomTexts?.[`__injected_${injIdx}`] || '';
    }

    // 3. Fall back to network fetch with scoped auth cookies
    if (!text && header.sourceURL && /^https?:/i.test(header.sourceURL)) {
      try {
        const response = await fetch(header.sourceURL, { headers: authCookieHeader ? { cookie: authCookieHeader } : {} });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        text = await response.text();
      } catch (fetchErr) {
        // Try CSSOM text for sheets with a URL too (in case fetch is blocked)
        text = captures[0]?.cssomTexts?.[header.sourceURL] || '';
        if (!text) throw fetchErr;
      }
    }
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
      mediaQueries: [...new Set([...text.matchAll(/@media\s+([^{]+)/g)].map((match) => match[1].trim()))].slice(0, 500),
      keyframes: [...new Set([...text.matchAll(/@(?:-webkit-)?keyframes\s+([^\s{]+)/g)].map((match) => match[1].trim()))].slice(0, 500),
      pseudoSelectors: [...new Set([...text.matchAll(/([^{}]+:(?:hover|active|focus-visible|focus|disabled|checked|expanded)[^{}]*)\{/g)].map((match) => match[1].trim()).filter((selector) => selector.length < 500))].slice(0, 1000),
      customProperties: [...new Set([...text.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]))].slice(0, 2000),
    });
  } catch (error) {
    stylesheetManifest.push({ sourceURL: header.sourceURL, error: String(error) });
  }
}
const scriptDir = path.join(outDir, 'scripts');
if (fullProfile) fs.mkdirSync(scriptDir, { recursive: true });
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
    if (fullProfile) {
      fs.writeFileSync(path.join(scriptDir, filename), liveScript.source);
    }
    sourceByScriptId.set(scriptId, {
      scriptId,
      source: liveScript.source,
      filename,
      lines: liveScript.source.split(/\r?\n/),
    });
    scriptManifest.push({
      scriptId,
      file: fullProfile ? `scripts/${filename}` : undefined,
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
    // Use the existing cdp connection — opening a second connection to the same tab kills the first
    const source = (await cdp.send('Debugger.getScriptSource', { scriptId }))
      .scriptSource;
    const filename = `${scriptId.replace(/[^\w.-]+/g, '_')}.js`;
    if (fullProfile) fs.writeFileSync(path.join(scriptDir, filename), source);
    sourceByScriptId.set(scriptId, {
      scriptId,
      source,
      filename,
      lines: source.split(/\r?\n/),
    });
    const parsed = scripts.get(scriptId);
    scriptManifest.push({
      scriptId,
      file: fullProfile ? `scripts/${filename}` : undefined,
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
      const response = await fetch(parsed.url, { headers: authCookieHeader ? { cookie: authCookieHeader } : {} });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const source = await response.text();
      const filename = `${scriptId.replace(/[^\w.-]+/g, '_')}.js`;
      if (fullProfile) fs.writeFileSync(path.join(scriptDir, filename), source);
      sourceByScriptId.set(scriptId, {
        scriptId,
        source,
        filename,
        lines: source.split(/\r?\n/),
      });
      scriptManifest.push({
        scriptId,
        file: fullProfile ? `scripts/${filename}` : undefined,
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
      if (!fullProfile) delete listener.sourceFile;
      listener.sourceUrl = scripts.get(listener.scriptId)?.url;
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
        sourceFile: fullProfile ? `scripts/${script.filename}` : undefined,
        sourceUrl: scripts.get(script.scriptId)?.url,
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
      sourceFile: fullProfile ? `scripts/${script.filename}` : undefined,
      sourceUrl: scripts.get(scriptId)?.url,
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
          sourceFile: fullProfile ? `scripts/${script.filename}` : undefined,
          sourceUrl: scripts.get(script.scriptId)?.url,
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
  profile,
  source: {
    requestedUrl,
    capturedUrl: captures[0]?.document.url,
    reusedAuthenticatedTarget: reuse,
    capturedAt: new Date().toISOString(),
  },
  home: homePageState,
  pages: multiPageStates,
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
if (fullProfile) fs.mkdirSync(componentDir, { recursive: true });
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
    if (fullProfile) {
      fs.writeFileSync(
        path.join(componentDir, filename),
        JSON.stringify(component, null, 2),
      );
    }
    return {
      id: component.id,
      file: fullProfile ? `components/${filename}` : undefined,
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
for (const component of componentPackages.filter((item) => item.file)) {
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

if (!fullProfile) {
  output.captures = captures.map(
    ({ documents: _documents, componentCandidates: _componentCandidates, ...capture }) =>
      capture,
  );
}

fs.writeFileSync(path.join(outDir, 'spec.json'), JSON.stringify(output, null, 2));
fs.writeFileSync(
  path.join(outDir, 'summary.json'),
  JSON.stringify(
    {
      source: output.source,
      profile,
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

try {
  cdp.close();
} catch {}
try {
  browser?.close();
} catch {}

console.log(fs.readFileSync(path.join(outDir, 'summary.json'), 'utf8'));
process.exit(0);
