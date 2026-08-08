// Drives the shipped settle script over a scripted page on a virtual clock, so the whole
// scenario runs in microtasks with no browser and no real elapsed time.
//
// The scene is a list of steps. One step is consumed per animation frame and the last step
// repeats forever, which lets a test describe a page that mutates for a while, moves for a
// while, and then holds still, without any timing assumptions.

const scene = __SCENE__;

let now = 0;
let frameIndex = 0;
let observers = [];

const stepAt = (index) => scene.steps[Math.min(index, scene.steps.length - 1)];
const current = () => stepAt(frameIndex);

Date.now = () => now;

globalThis.MutationObserver = class {
  constructor(callback) {
    this.callback = callback;
  }
  observe() {
    observers.push(this);
  }
  disconnect() {
    observers = observers.filter((entry) => entry !== this);
  }
};

globalThis.requestAnimationFrame = (callback) => {
  queueMicrotask(() => {
    now += 16;
    frameIndex++;
    if (current().mutate) {
      for (const observer of observers) observer.callback([]);
    }
    callback(now);
  });
};

globalThis.setTimeout = (callback, delay) => {
  queueMicrotask(() => {
    now += delay || 0;
    callback();
  });
};

const DEFAULTS = {
  display: "block",
  visibility: "visible",
  opacity: "1",
  position: "static",
  "z-index": "auto",
  "pointer-events": "auto",
};

// Element identity is keyed by position in the step, not by the step's own object, so an
// element that moves between frames is still recognisably the same element — which is what
// lets an animation point at one and the script match it against the scanned page.
const cache = new Map();

const element = (spec, index) => {
  if (!cache.has(index)) cache.set(index, {});
  const node = cache.get(index);
  node.tagName = (spec.tag || "div").toUpperCase();
  node.getBoundingClientRect = () => spec.rect;
  node.declarations = { ...DEFAULTS, ...(spec.style || {}) };
  return node;
};

const animation = (spec) =>
  Object.assign(spec.declared ? new CSSAnimation() : {}, {
    playState: spec.playState || "running",
    effect: {
      target: element((current().elements || [])[spec.element], spec.element),
      getComputedTiming: () => ({
        delay: spec.delay || 0,
        duration: spec.duration || 0,
        localTime: spec.localTime || 0,
      }),
    },
  });

class CSSAnimation {}
globalThis.CSSAnimation = CSSAnimation;

globalThis.innerWidth = 1000;
globalThis.innerHeight = 1000;

globalThis.getComputedStyle = (target) =>
  new Proxy(
    {},
    {
      get: (_, name) =>
        target.declarations[
          String(name).replace(/[A-Z]/g, (letter) => "-" + letter.toLowerCase())
        ],
    },
  );

globalThis.document = {
  get readyState() {
    return current().loading ? "loading" : "complete";
  },
  fonts: {
    get status() {
      return current().fontsPending ? "loading" : "loaded";
    },
  },
  querySelectorAll: () => (current().elements || []).map(element),
  getAnimations: () => (current().animations || []).map(animation),
};

globalThis.window = globalThis;
globalThis.__recreatePendingRequests = 0;
globalThis.__recreateLifecycleDone = !scene.lifecyclePending;

__SETTLE__.then((resolved) => {
  console.log(JSON.stringify({ resolved, elapsed: now, frames: frameIndex }));
});
