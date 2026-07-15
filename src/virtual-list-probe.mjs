const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const evaluate = async (cdp, expression) =>
  (
    await cdp.send('Runtime.evaluate', {
      expression,
      returnByValue: true,
    })
  ).result.value;
const dispatchKey = async (cdp, key, code, keyCode) => {
  for (const type of ['keyDown', 'keyUp']) {
    await cdp.send('Input.dispatchKeyEvent', {
      type,
      key,
      code,
      keyCode,
      windowsVirtualKeyCode: keyCode,
    });
  }
};

const discoveryExpression = `(() => {
  const roots = [document];
  while (roots.length) {
    const root = roots.shift();
    for (const listbox of root.querySelectorAll(
      '[role="listbox"][aria-activedescendant]'
    )) {
      const options = Array.from(listbox.querySelectorAll('[role="option"]'));
      const logicalSize = Math.max(
        Number(listbox.getAttribute('aria-rowcount') || 0),
        ...options.map(option => Number(option.getAttribute('aria-setsize') || 0))
      );
      if (
        logicalSize > options.length &&
        listbox.scrollHeight > listbox.clientHeight
      ) {
        const activeId = listbox.getAttribute('aria-activedescendant');
        const active = options.find(option => option.id === activeId);
        const activePosition = Number(active?.getAttribute('aria-posinset') || 0);
        if (activePosition !== 1 || listbox.scrollTop !== 0) continue;
        let previousFocus = document.activeElement;
        while (previousFocus?.shadowRoot?.activeElement) {
          previousFocus = previousFocus.shadowRoot.activeElement;
        }
        window.__recreateVirtualList = listbox;
        window.__recreateVirtualPreviousFocus = previousFocus;
        return {
          label: listbox.getAttribute('aria-label') || 'Virtual listbox',
          tag: listbox.tagName.toLowerCase(),
          logicalSize,
          mounted: options.map(option => ({
            id: option.id,
            position: Number(option.getAttribute('aria-posinset') || 0),
            selected: option.getAttribute('aria-selected') === 'true',
            text: (option.innerText || '').trim()
          })),
          activeDescendant: listbox.getAttribute('aria-activedescendant'),
          scrollTop: listbox.scrollTop,
          clientHeight: listbox.clientHeight,
          scrollHeight: listbox.scrollHeight
        };
      }
    }
    for (const element of root.querySelectorAll('*')) {
      if (element.shadowRoot) roots.push(element.shadowRoot);
    }
  }
  return null;
})()`;

const stateExpression = `(() => {
  const listbox = window.__recreateVirtualList;
  if (!listbox) return null;
  return {
    activeDescendant: listbox.getAttribute('aria-activedescendant'),
    scrollTop: listbox.scrollTop,
    clientHeight: listbox.clientHeight,
    scrollHeight: listbox.scrollHeight,
    mounted: Array.from(listbox.querySelectorAll('[role="option"]')).map(
      option => ({
        id: option.id,
        position: Number(option.getAttribute('aria-posinset') || 0),
        selected: option.getAttribute('aria-selected') === 'true',
        text: (option.innerText || '').trim()
      })
    ),
    focus: (() => {
      let element = document.activeElement;
      while (element?.shadowRoot?.activeElement) {
        element = element.shadowRoot.activeElement;
      }
      return element && element !== document.body
        ? { tag: element.tagName.toLowerCase(), id: element.id || null }
        : null;
    })()
  };
})()`;

async function restoreVirtualList(cdp) {
  await dispatchKey(cdp, 'Home', 'Home', 36);
  await delay(50);
  await evaluate(cdp, `(() => {
    const listbox = window.__recreateVirtualList;
    const previous = window.__recreateVirtualPreviousFocus;
    if (previous && previous !== document.body) previous.focus();
    else listbox?.blur();
  })()`);
  const restored = await evaluate(cdp, stateExpression);
  await evaluate(cdp, `delete window.__recreateVirtualList;
    delete window.__recreateVirtualPreviousFocus`);
  return restored;
}

export async function captureVirtualListState({
  cdp,
  maxStates,
  states,
  viewports,
  capturePageSnapshot,
}) {
  if (states.length >= maxStates) return;
  const before = await evaluate(cdp, discoveryExpression);
  if (!before) return;
  await evaluate(cdp, `window.__recreateVirtualList.focus()`);
  await dispatchKey(cdp, 'ArrowDown', 'ArrowDown', 40);
  await dispatchKey(cdp, 'End', 'End', 35);
  await delay(50);
  const after = await evaluate(cdp, stateExpression);
  if (!after || after.activeDescendant === before.activeDescendant) {
    await restoreVirtualList(cdp);
    return;
  }

  const viewport = viewports[0];
  const index = states.length;
  const state = await capturePageSnapshot(
    index,
    `${String(index).padStart(3, '0')}-virtual-list`,
    viewport,
    true,
  );
  Object.assign(state, {
    type: 'virtual-list',
    trigger: before.label,
    triggerElement: { label: before.label, tag: before.tag, role: 'listbox' },
    probe: {
      sequence: [
        { action: 'focus' },
        { action: 'key', key: 'ArrowDown' },
        { action: 'key', key: 'End' },
      ],
    },
    virtualization: {
      logicalSize: before.logicalSize,
      before: {
        mounted: before.mounted,
        activeDescendant: before.activeDescendant,
        scrollTop: before.scrollTop,
      },
      after,
    },
    evidenceByViewport: {
      [`${viewport.width}x${viewport.height}`]: state.evidence,
    },
  });
  states.push(state);
  state.virtualization.restored = await restoreVirtualList(cdp);
}
