  // The address every capture pass gives an element, defined once.
  //
  // A path is a key in one map: the resting capture, the interaction capture, the lifecycle
  // recorder and the comparison passes all write and read it, so they must agree byte for
  // byte or the artifact silently associates one element's record with another's.
  //
  // `parentElement` is null whenever the parent node is not an element, which is true of
  // every top node of a shadow tree, so it cannot distinguish "the document root" from
  // "the boundary of a shadow tree". `getRootNode()` is the value that can: it answers for
  // every node, and it is a `ShadowRoot` exactly inside a shadow tree. The host's own path
  // prefixes the tree it opens, because a shadow tree numbers its children from one and a
  // path that stopped at the boundary would repeat for every host on the page.
  const pathCache = new WeakMap([[document.documentElement, 'html']]);
  const siblingIndexes = new WeakMap();
  const siblingIndex = element => {
    const root = element.parentElement || element.getRootNode();
    let indexes = siblingIndexes.get(root);
    if (!indexes) {
      indexes = new WeakMap();
      const counts = new Map();
      for (const child of root.children) {
        const count = (counts.get(child.tagName) || 0) + 1;
        counts.set(child.tagName, count);
        indexes.set(child, count);
      }
      siblingIndexes.set(root, indexes);
    }
    return indexes.get(element) || 1;
  };
  const shadowPath = root => `${pathOf(root.host)}>::shadow-root(${root.mode})`;
  // The address of the node that holds `element`. Every record of parentage is this value,
  // so it is written once: a copy that stops at `parentElement` reports no holder at all for
  // the top of a shadow tree, which detaches that whole tree from the element that opened it.
  // `null` is reserved for the one element that genuinely has no holder.
  const holderPath = element => {
    if (element.parentElement) return pathOf(element.parentElement);
    const root = element.getRootNode();
    return root instanceof ShadowRoot ? shadowPath(root) : null;
  };
  const pathOf = element => {
    const cached = pathCache.get(element);
    if (cached) return cached;
    const parent = holderPath(element) ?? 'html';
    const path = `${parent}>${element.tagName.toLowerCase()}:nth-of-type(${siblingIndex(element)})`;
    pathCache.set(element, path);
    return path;
  };
