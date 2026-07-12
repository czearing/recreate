import {
  cleanupHoverExpression,
  discoverHoverExpression,
  finishHoverLeaveExpression,
  hoverAnimationExpression,
  hoverStateExpression,
  prepareHoverExpression,
} from './hover-runtime.mjs';
import { hoverRuleExpression } from './hover-style-runtime.mjs';

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const evaluate = async (cdp, expression) =>
  (
    await cdp.send('Runtime.evaluate', {
      expression,
      returnByValue: true,
    })
  ).result.value;
const move = (cdp, x, y) =>
  cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });

const restored = (before, after) =>
  Math.abs(before.rect.x - after.rect.x) <= 0.1 &&
  Math.abs(before.rect.y - after.rect.y) <= 0.1 &&
  before.style.opacity === after.style.opacity &&
  before.style.transform === after.style.transform &&
  before.style.filter === after.style.filter &&
  before.style.boxShadow === after.style.boxShadow &&
  before.style.backgroundColor === after.style.backgroundColor &&
  JSON.stringify(before.scroll) === JSON.stringify(after.scroll);

export async function captureHoverState({
  cdp,
  match,
  maxStates,
  states,
  viewports,
  capturePageSnapshot,
}) {
  if (states.length >= maxStates) return;
  const descriptor = await evaluate(cdp, discoverHoverExpression(match));
  if (!descriptor) return;
  await evaluate(cdp, prepareHoverExpression);
  await delay(20);
  const before = await evaluate(cdp, hoverStateExpression);
  let cleaned = false;
  try {
    await move(cdp, 0, 0);
    await move(cdp, descriptor.x, descriptor.y);
    await delay(20);
    const rules = await evaluate(cdp, hoverRuleExpression);
    const animations = await evaluate(cdp, hoverAnimationExpression);
    const after = await evaluate(cdp, hoverStateExpression);
    if (!animations.length && restored(before, after)) return;

    const viewport = viewports[0];
    const index = states.length;
    const state = await capturePageSnapshot(
      index,
      `${String(index).padStart(3, '0')}-hover`,
      viewport,
      true,
      false,
    );
    Object.assign(state, {
      type: 'hover',
      trigger: descriptor.label,
      triggerElement: {
        label: descriptor.label,
        tag: descriptor.tag,
      },
      probe: {
        action: 'hover',
        point: { x: descriptor.x, y: descriptor.y },
      },
      hoverInteraction: {
        before,
        after,
        animations,
        authoredRules: rules,
      },
      evidenceByViewport: {
        [`${viewport.width}x${viewport.height}`]: state.evidence,
      },
    });
    states.push(state);

    await move(cdp, 0, 0);
    await delay(20);
    await evaluate(cdp, finishHoverLeaveExpression);
    state.hoverInteraction.restored = await evaluate(cdp, hoverStateExpression);
    if (!restored(before, state.hoverInteraction.restored)) {
      throw new Error(`Hover probe failed to restore: ${JSON.stringify({
        before,
        restored: state.hoverInteraction.restored,
      })}`);
    }
    const cleanup = await evaluate(cdp, cleanupHoverExpression);
    cleaned = true;
    if (!cleanup.scrollRestored) {
      throw new Error('Hover probe failed to restore original scroll positions');
    }
  } finally {
    await move(cdp, 0, 0).catch(() => {});
    if (!cleaned) {
      await evaluate(cdp, cleanupHoverExpression).catch(() => {});
    }
  }
}
