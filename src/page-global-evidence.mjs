const clean = (value) => String(value || '').replace(/\s+/g, ' ').trim();
const within = (pathValue, rootPath) =>
  pathValue === rootPath || pathValue?.startsWith(`${rootPath}>`);

export function buildPageGlobalContent(nodes, componentRoots) {
  const childCounts = new Map();
  for (const node of nodes) {
    if (node.parentPath) {
      childCounts.set(node.parentPath, (childCounts.get(node.parentPath) || 0) + 1);
    }
  }
  return nodes
    .filter((node) => {
      const text = clean(node.text);
      return (
        text &&
        text.length <= 300 &&
        !childCounts.get(node.path) &&
        node.rect?.width > 0 &&
        node.rect?.height > 0 &&
        !componentRoots.some((root) => within(node.path, root))
      );
    })
    .map((node) => ({
      path: node.path,
      parentPath: node.parentPath,
      tag: node.tag,
      text: clean(node.text),
      rect: node.rect,
      style: node.style,
    }));
}

export function buildPageGlobalLayout(nodes, componentRoots, content) {
  const byPath = new Map(nodes.map((node) => [node.path, node]));
  const selected = new Map();
  for (const leaf of content) {
    let parentPath = leaf.parentPath;
    for (let depth = 0; parentPath && depth < 4; depth += 1) {
      const node = byPath.get(parentPath);
      if (!node || componentRoots.some((root) => within(node.path, root))) break;
      selected.set(node.path, {
        path: node.path,
        parentPath: node.parentPath,
        tag: node.tag,
        rect: node.rect,
        style: node.style,
      });
      parentPath = node.parentPath;
    }
  }
  return [...selected.values()];
}
