import {
  cleanupIframeExpression,
  discoverIframeExpression,
  iframeStateExpression,
} from './iframe-runtime.mjs';

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const evaluate = async (cdp, expression) =>
  (
    await cdp.send('Runtime.evaluate', {
      expression,
      returnByValue: true,
    })
  ).result.value;
const click = async (cdp, point) => {
  await cdp.send('Input.dispatchMouseEvent', {
    type: 'mouseMoved',
    ...point,
  });
  await cdp.send('Input.dispatchMouseEvent', {
    type: 'mousePressed',
    ...point,
    button: 'left',
    buttons: 1,
    clickCount: 1,
  });
  await cdp.send('Input.dispatchMouseEvent', {
    type: 'mouseReleased',
    ...point,
    button: 'left',
    buttons: 0,
    clickCount: 1,
  });
};
const frameUrls = (frameTree) => {
  const values = [];
  const visit = (entry) => {
    values.push(entry.frame.url);
    for (const child of entry.childFrames || []) visit(child);
  };
  visit(frameTree);
  return values;
};

export async function captureIframeState({
  cdp,
  maxStates,
  states,
  viewports,
  capturePageSnapshot,
  cleanupPage,
}) {
  if (states.length >= maxStates) return;
  const descriptor = await evaluate(cdp, discoverIframeExpression);
  if (!descriptor?.sameOrigin || !descriptor.button) return;
  const point = { x: descriptor.button.x, y: descriptor.button.y };
  const before = await evaluate(cdp, iframeStateExpression);
  const protocolBefore = frameUrls(
    (await cdp.send('Page.getFrameTree')).frameTree,
  );
  let reloadRequired = false;
  let capturedState;
  try {
    await click(cdp, point);
    await delay(100);
    const after = await evaluate(cdp, iframeStateExpression);
    const protocolAfter = frameUrls(
      (await cdp.send('Page.getFrameTree')).frameTree,
    );
    if (
      !after ||
      (
        !after.unavailable &&
        after.parentStatus === before.parentStatus &&
        JSON.stringify(after.nodes) === JSON.stringify(before.nodes)
      )
    ) return;

    const viewport = viewports[0];
    const index = states.length;
    const state = await capturePageSnapshot(
      index,
      `${String(index).padStart(3, '0')}-iframe`,
      viewport,
      true,
      false,
    );
    Object.assign(state, {
      type: 'iframe',
      trigger: descriptor.button.label,
      triggerElement: {
        label: descriptor.button.label,
        tag: 'button',
        frame: descriptor.sameOrigin.src,
      },
      probe: { action: 'frame-click', point },
      iframeInteraction: {
        boundaries: descriptor.frames,
        protocolFrames: { before: protocolBefore, after: protocolAfter },
        before,
        after,
      },
      evidenceByViewport: {
        [`${viewport.width}x${viewport.height}`]: state.evidence,
      },
    });
    states.push(state);
    capturedState = state;
    reloadRequired = true;
  } finally {
    await evaluate(cdp, cleanupIframeExpression).catch(() => {});
    if (reloadRequired) {
      await cleanupPage();
      await delay(100);
      const restoredDescriptor = await evaluate(cdp, discoverIframeExpression);
      if (restoredDescriptor?.sameOrigin) {
        const restored = await evaluate(cdp, iframeStateExpression);
        if (capturedState) capturedState.iframeInteraction.restored = restored;
        await evaluate(cdp, cleanupIframeExpression).catch(() => {});
        if (
          restored.parentStatus !== before.parentStatus ||
          JSON.stringify(restored.nodes) !== JSON.stringify(before.nodes)
        ) {
          throw new Error('Iframe probe failed to restore child state.');
        }
      } else {
        throw new Error('Iframe probe could not restore the same-origin frame.');
      }
    }
  }
}
