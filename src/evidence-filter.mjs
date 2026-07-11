const adIdentityPattern =
  /(?:\badunit|advertisement|cmpwrapper|mv-ad|privacy[-_ ]?(?:choice|manager)|consent[-_ ]?manager)/i;

export function implementationNodes(nodes) {
  const mainDocumentNodes = nodes.filter((node) =>
    node.path.startsWith('doc(0)>'),
  );
  const excludedRoots = mainDocumentNodes
    .filter((node) => {
      const attrs = node.attrs || {};
      const identity = [
        node.tag,
        attrs.id,
        attrs.class,
        attrs['aria-label'],
        node.text,
      ].join(' ');
      return adIdentityPattern.test(identity);
    })
    .map((node) => node.path);
  const withoutAds = mainDocumentNodes.filter(
    (node) =>
      !excludedRoots.some(
        (root) => node.path === root || node.path.startsWith(`${root}>`),
      ),
  );
  const byPath = new Map(withoutAds.map((node) => [node.path, node]));
  const keptPaths = new Set();
  const keepWithAncestors = (node) => {
    let current = node;
    while (current && !keptPaths.has(current.path)) {
      keptPaths.add(current.path);
      current = byPath.get(current.parentPath);
    }
  };
  for (const node of withoutAds) {
    const attrs = node.attrs || {};
    if (
      node.visible ||
      node.clickable ||
      node.tag === 'html' ||
      node.tag === 'body' ||
      attrs.role ||
      attrs['aria-label'] ||
      attrs['data-testid']
    ) {
      keepWithAncestors(node);
    }
  }
  return withoutAds.filter((node) => keptPaths.has(node.path));
}

const compactListener = (listener) => ({
  target: listener.target,
  type: listener.type,
  capture: listener.capture || undefined,
  passive: listener.passive || undefined,
  once: listener.once || undefined,
  sourceUrl: listener.sourceUrl,
  lineNumber: listener.lineNumber >= 0 ? listener.lineNumber : undefined,
  columnNumber: listener.columnNumber >= 0 ? listener.columnNumber : undefined,
  sourceStatus: listener.sourceStatus,
});

export function compactListeners(listeners = []) {
  const unique = new Map();
  for (const listener of listeners) {
    if (
      /(?:ampproject|doubleclick|googleads|googlesyndication|pubnation|adnxs|amazon-adsystem)/i
        .test(listener.sourceUrl || '')
    ) {
      continue;
    }
    const compact = compactListener(listener);
    const key = JSON.stringify(compact);
    if (!unique.has(key)) unique.set(key, compact);
  }
  return [...unique.values()];
}
