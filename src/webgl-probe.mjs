import {
  cleanupWebglExpression,
  discoverWebglCanvasExpression,
  webglStepSignatureExpression,
} from './webgl-runtime.mjs';

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const evaluate = async (cdp, expression) =>
  (
    await cdp.send('Runtime.evaluate', {
      expression,
      returnByValue: true,
    })
  ).result.value;
const point = (rect, offset = 0) => ({
  x: rect.x + rect.width / 2 + offset,
  y: rect.y + rect.height / 2,
});
const mouse = (cdp, type, position, extra = {}) =>
  cdp.send('Input.dispatchMouseEvent', { type, ...position, ...extra });

async function drag(cdp, from, to) {
  await mouse(cdp, 'mouseMoved', from);
  await mouse(cdp, 'mousePressed', from, {
    button: 'left',
    buttons: 1,
    clickCount: 1,
  });
  try {
    await mouse(cdp, 'mouseMoved', to, { button: 'left', buttons: 1 });
    await delay(30);
  } finally {
    await mouse(cdp, 'mouseReleased', to, {
      button: 'left',
      buttons: 0,
      clickCount: 1,
    });
  }
}

const distance = (left, right) => {
  if (!left?.sampledGrid || !right?.sampledGrid) return undefined;
  let total = 0;
  let count = 0;
  left.sampledGrid.forEach((sample, index) => {
    sample.forEach((value, channel) => {
      total += Math.abs(value - right.sampledGrid[index][channel]);
      count++;
    });
  });
  return total / count;
};

export async function captureWebglInteractionState({
  cdp,
  maxStates,
  states,
  viewports,
  capturePageSnapshot,
  cleanupPage,
}) {
  try {
    if (states.length >= maxStates) return;
    const descriptor = await evaluate(cdp, discoverWebglCanvasExpression);
    if (!descriptor) return;
    await evaluate(cdp, `window.__recreateRafControl.pause()`);
    let pauseStatus;
    for (let attempt = 0; attempt < 20; attempt++) {
      pauseStatus = await evaluate(
        cdp,
        `window.__recreateRafControl.status()`,
      );
      if (pauseStatus.queued) break;
      await delay(20);
    }
    const initial = await evaluate(cdp, webglStepSignatureExpression(0));
    await delay(40);
    const phase = await evaluate(cdp, webglStepSignatureExpression(250));
    await delay(40);
    const control = await evaluate(cdp, webglStepSignatureExpression(0));
    await delay(40);

    const center = point(descriptor.rect);
    const moved = point(descriptor.rect, Math.min(100, descriptor.rect.width / 4));
    await drag(cdp, center, moved);
    const interacted = await evaluate(cdp, webglStepSignatureExpression(0));
    await delay(40);
    await drag(cdp, moved, center);
    const restored = await evaluate(cdp, webglStepSignatureExpression(0));
    await delay(40);

    const index = states.length;
    const viewport = viewports[0];
    const state = await capturePageSnapshot(
      index,
      `${String(index).padStart(3, '0')}-webgl`,
      viewport,
      true,
      false,
    );
    Object.assign(state, {
      type: 'webgl-interaction',
      trigger: descriptor.label,
      probe: {
        sequence: [
          { action: 'pause-animation' },
          { action: 'step-animation', deltaMs: 250 },
          { action: 'step-animation', deltaMs: 0, purpose: 'control' },
          { action: 'pointer-drag', from: center, to: moved },
          { action: 'pointer-drag', from: moved, to: center },
        ],
      },
      webglInteraction: {
        initial,
        phase,
        control,
        interacted,
        restored,
        phaseDistance: distance(initial, phase),
        controlDistance: distance(phase, control),
        interactionDistance: distance(control, interacted),
        restorationDistance: distance(control, restored),
        restorationExact: control.sampleHash === restored.sampleHash,
      },
      evidenceByViewport: {
        [`${viewport.width}x${viewport.height}`]: state.evidence,
      },
    });
    states.push(state);
    if (
      initial.sampleHash === phase.sampleHash ||
      phase.sampleHash !== control.sampleHash ||
      control.sampleHash === interacted.sampleHash
    ) {
      throw new Error(`WebGL invariant failed: ${JSON.stringify({
        initial: initial.sampleHash,
        phase: phase.sampleHash,
        control: control.sampleHash,
        interacted: interacted.sampleHash,
        restored: restored.sampleHash,
        pauseStatus,
        callbacks: [
          initial?.callbacks,
          phase?.callbacks,
          control?.callbacks,
          interacted?.callbacks,
          restored?.callbacks,
        ],
        means: [
          initial?.meanRgba,
          phase?.meanRgba,
          control?.meanRgba,
          interacted?.meanRgba,
          restored?.meanRgba,
        ],
      })}`);
    }
  } finally {
    await evaluate(cdp, cleanupWebglExpression).catch(() => {});
    await cleanupPage();
  }
}
