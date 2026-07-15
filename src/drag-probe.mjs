const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const evaluate = async (cdp, expression) =>
  (
    await cdp.send('Runtime.evaluate', {
      expression,
      returnByValue: true,
    })
  ).result.value;
const point = (rect) => ({
  x: rect.x + rect.width / 2,
  y: rect.y + rect.height / 2,
});
const mouse = (cdp, type, position, extra = {}) =>
  cdp.send('Input.dispatchMouseEvent', {
    type,
    ...position,
    ...extra,
  });

async function performDrag(cdp, sourceRect, targetRect, hold = false) {
  const source = point(sourceRect);
  const target = point(targetRect);
  let pressed = false;
  try {
    await mouse(cdp, 'mouseMoved', source);
    await mouse(cdp, 'mousePressed', source, {
      button: 'left',
      buttons: 1,
      clickCount: 1,
    });
    pressed = true;
    await mouse(cdp, 'mouseMoved', target, { button: 'left', buttons: 1 });
    await delay(50);
    if (!hold) {
      await mouse(cdp, 'mouseReleased', target, {
        button: 'left',
        buttons: 0,
        clickCount: 1,
      });
      pressed = false;
    }
    return { source, target };
  } catch (error) {
    if (pressed) {
      await mouse(cdp, 'mouseReleased', source, {
        button: 'left',
        buttons: 0,
      }).catch(() => {});
    }
    throw error;
  }
}

async function restoreFocus(cdp) {
  return evaluate(cdp, `(() => {
    const previous = window.__recreateDragPreviousFocus;
    if (previous && previous !== document.body) previous.focus();
    else document.activeElement?.blur?.();
    delete window.__recreateDragPreviousFocus;
    delete window.__recreateDragSource;
    delete window.__recreateDragTarget;
    delete window.__recreateDragIdentities;
    let focus = document.activeElement;
    while (focus?.shadowRoot?.activeElement) {
      focus = focus.shadowRoot.activeElement;
    }
    return focus && focus !== document.body
      ? { tag: focus.tagName.toLowerCase(), id: focus.id || null }
      : null;
  })()`);
}

export async function captureDragStates({
  cdp,
  maxStates,
  states,
  viewports,
  capturePageSnapshot,
}) {
  if (maxStates - states.length < 2) return;
  const before = await evaluate(cdp, discoverDragExpression);
  if (!before) return;
  let pointer;
  let pointerHeld = false;
  let focusRestored = false;
  try {
    pointer = await performDrag(
      cdp,
      before.sourceRect,
      before.targetRect,
      true,
    );
    pointerHeld = true;
    const intermediate = await evaluate(cdp, dragStateExpression);
    if (!intermediate.items.some((item) => item.dragging || item.dropTarget)) {
      return;
    }

    const viewport = viewports[0];
    const transientIndex = states.length;
    const transient = await capturePageSnapshot(
      transientIndex,
      `${String(transientIndex).padStart(3, '0')}-drag`,
      viewport,
      true,
      false,
    );
    Object.assign(transient, {
      type: 'drag-transient',
      trigger: before.sourceId,
      probe: { action: 'pointer-drag', ...pointer },
      drag: { before, intermediate },
      evidenceByViewport: {
        [`${viewport.width}x${viewport.height}`]: transient.evidence,
      },
    });
    states.push(transient);

    await mouse(cdp, 'mouseReleased', pointer.target, {
      button: 'left',
      buttons: 0,
      clickCount: 1,
    });
    pointerHeld = false;
    await delay(200);
    const after = await evaluate(cdp, dragStateExpression);
    const finalIndex = states.length;
    const final = await capturePageSnapshot(
      finalIndex,
      `${String(finalIndex).padStart(3, '0')}-drop`,
      viewport,
      true,
    );
    Object.assign(final, {
      type: 'drop',
      trigger: before.sourceId,
      probe: { action: 'pointer-release', ...pointer },
      drag: { before, intermediate, after },
      evidenceByViewport: {
        [`${viewport.width}x${viewport.height}`]: final.evidence,
      },
    });
    states.push(final);

    const restoreRects = await evaluate(cdp, restoreDragRectsExpression);
    await performDrag(cdp, restoreRects.targetRect, restoreRects.sourceRect);
    await delay(200);
    final.drag.restored = await evaluate(cdp, dragStateExpression);
    final.drag.restoredFocus = await restoreFocus(cdp);
    focusRestored = true;
    if (final.drag.restored.order.join('|') !== before.order.join('|')) {
      throw new Error('Drag probe failed to restore the original item order.');
    }
  } finally {
    if (pointerHeld && pointer) {
      await mouse(cdp, 'mouseReleased', pointer.source, {
        button: 'left',
        buttons: 0,
      }).catch(() => {});
    }
    if (!focusRestored) {
      await restoreFocus(cdp).catch(() => {});
    }
  }
}
import {
  discoverDragExpression,
  dragStateExpression,
  restoreDragRectsExpression,
} from './drag-runtime.mjs';
