export function anchorParent(root, trigger) {
  const active = root.querySelector('[data-recreate-active="true"]');
  if (active?.parentElement) return prepareAnchor(root, active.parentElement);
  const fallback = root.querySelector(`[data-recreate-trigger="${trigger}"]`);
  return fallback?.parentElement
    ? prepareAnchor(root, fallback.parentElement)
    : root.body;
}

function prepareAnchor(root, parent) {
  if (root.defaultView?.getComputedStyle(parent).position === 'static') {
    parent.style.position = 'relative';
  }
  return parent;
}
