export function anchorParent(root, trigger) {
  const active = root.querySelector('[data-recreate-active="true"]');
  if (active?.parentElement) return active.parentElement;
  const fallback = root.querySelector(`[data-recreate-trigger="${trigger}"]`);
  return fallback?.parentElement || root.body;
}
