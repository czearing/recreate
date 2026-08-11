// Drives the shipped observation script over a scripted mutation timeline on a virtual
// clock, so a scenario that spans the full twelve-second ceiling runs in microtasks with no
// browser and no real elapsed time.
//
// The scene is a list of frames. Each frame appends its attribute events to the page's
// recorded list before the script's next animation frame runs, so a test describes what the
// page does and asserts only when the script let go.

const scene = __SCENE__;
const FRAME_MS = 16;

globalThis.window = globalThis;
// What the recorder had already written when it handed over. The observer never attaches at
// a page's first change — the recorder starts at document start and runs until it settles —
// so a timeline with nothing behind it describes a page nothing has happened on yet, and the
// mid-gap attach that every real capture makes could not be expressed at all.
const history = __HISTORY__;
window.__recreateAttributeMutations = history;
// The recorder stamps every change and records where its own clock started, and the observer
// reads both, so a harness that omitted them would exercise a rule the browser never runs.
window.__recreateLifecycleStart = 0;

let now = history.reduce((latest, event) => Math.max(latest, event.time), 0);
let frames = 0;

Date.now = () => now;
globalThis.performance = { now: () => now };

globalThis.requestAnimationFrame = (callback) => {
  queueMicrotask(() => {
    now += FRAME_MS;
    for (const event of scene[frames] || []) {
      window.__recreateAttributeMutations.push({ time: now, ...event });
    }
    frames++;
    callback(now);
  });
};

__SCRIPT__.then((elapsed) => console.log(JSON.stringify({ elapsed, frames })));
