export const discoverDragExpression = `(() => {
  const roots = [document];
  const items = [];
  while (roots.length) {
    const root = roots.shift();
    items.push(...root.querySelectorAll(
      '[data-draggable="true"],[draggable="true"]'
    ));
    for (const element of root.querySelectorAll('*')) {
      if (element.shadowRoot) roots.push(element.shadowRoot);
    }
  }
  const visible = items.filter(element => {
    const rect = element.getBoundingClientRect();
    return rect.width > 4 && rect.height > 4;
  });
  const source = visible.find(element =>
    visible.some(candidate =>
      candidate !== element && candidate.parentElement === element.parentElement
    )
  );
  const target = visible.find(element =>
    element !== source && element.parentElement === source?.parentElement
  );
  if (!source || !target) return null;
  const siblings = Array.from(source.parentElement.children).filter(element =>
    element.matches('[data-draggable="true"],[draggable="true"]')
  );
  const identities = new Map(
    siblings.map((element, index) => [
      element,
      element.id ? '#' + element.id : 'recreate-drag-' + index
    ])
  );
  const identityFor = element => identities.get(element);
  let previousFocus = document.activeElement;
  while (previousFocus?.shadowRoot?.activeElement) {
    previousFocus = previousFocus.shadowRoot.activeElement;
  }
  window.__recreateDragPreviousFocus = previousFocus;
  window.__recreateDragSource = source;
  window.__recreateDragTarget = target;
  window.__recreateDragIdentities = identities;
  return {
    sourceId: identityFor(source),
    targetId: identityFor(target),
    sourceRect: source.getBoundingClientRect().toJSON(),
    targetRect: target.getBoundingClientRect().toJSON(),
    order: siblings.map(identityFor)
  };
})()`;

export const dragStateExpression = `(() => {
  const source = window.__recreateDragSource;
  if (!source) return null;
  const identityFor = element =>
    window.__recreateDragIdentities?.get(element);
  const items = Array.from(source.parentElement.children).filter(element =>
    element.matches('[data-draggable="true"],[draggable="true"]')
  );
  return {
    order: items.map(identityFor),
    items: items.map(element => ({
      id: identityFor(element),
      dragging: element.getAttribute('data-dragging') === 'true',
      dropTarget: element.getAttribute('data-drop-target') === 'true',
      rect: element.getBoundingClientRect().toJSON()
    }))
  };
})()`;

export const restoreDragRectsExpression = `(() => ({
  sourceRect: window.__recreateDragSource?.getBoundingClientRect().toJSON(),
  targetRect: window.__recreateDragTarget?.getBoundingClientRect().toJSON()
}))()`;
