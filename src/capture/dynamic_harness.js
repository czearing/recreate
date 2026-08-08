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
window.__recreateAttributeMutations = [];

let now = 0;
let frames = 0;

Date.now = () => now;

globalThis.requestAnimationFrame = (callback) => {
  queueMicrotask(() => {
    now += FRAME_MS;
    for (const event of scene[frames] || []) {
      window.__recreateAttributeMutations.push(event);
    }
    frames++;
    callback(now);
  });
};

__SCRIPT__.then((elapsed) => console.log(JSON.stringify({ elapsed, frames })));
