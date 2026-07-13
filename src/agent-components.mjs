const STYLE_KEYS = [
  'display', 'position', 'width', 'height', 'minWidth', 'maxWidth',
  'minHeight', 'maxHeight', 'margin', 'padding', 'gap', 'rowGap',
  'columnGap', 'flex', 'flexDirection', 'flexWrap', 'justifyContent',
  'alignItems', 'gridTemplateColumns', 'gridTemplateRows', 'overflow',
  'overflowX', 'overflowY', 'boxSizing', 'color', 'backgroundColor',
  'backgroundImage', 'border', 'borderRadius', 'boxShadow', 'opacity',
  'transform', 'fontFamily', 'fontSize', 'fontWeight', 'lineHeight',
  'letterSpacing', 'textAlign', 'whiteSpace',
];

const ATTR_KEYS = [
  'id', 'role', 'aria-label', 'aria-expanded', 'aria-pressed',
  'aria-selected', 'aria-haspopup', 'aria-controls', 'placeholder',
  'title', 'alt', 'href', 'type', 'data-testid',
];

const clean = (value, limit = 180) =>
  String(value || '').replace(/\s+/g, ' ').trim().slice(0, limit);

const isHashClass = (value) =>
  !value || /^sc-[a-z0-9]+$/i.test(value) || /^[a-z0-9_-]{12,}$/i.test(value);

const within = (pathValue, rootPath) =>
  pathValue === rootPath || pathValue?.startsWith(`${rootPath}>`);

const relativePath = (pathValue, rootPath) =>
  pathValue === rootPath ? '.' : pathValue.slice(rootPath.length + 1);

const pick = (source, keys) => Object.fromEntries(
  keys.filter((key) => source?.[key] != null && source[key] !== '')
    .map((key) => [key, source[key]]),
);

function nodeLabel(node, childPaths) {
  const attrs = node?.attrs || {};
  const explicit = node?.ariaLabel || attrs['aria-label'] || attrs.alt ||
    attrs.placeholder || attrs.title;
  if (explicit) return clean(explicit);
  const isLeaf = !childPaths.has(node.path);
  if (isLeaf || /^h[1-6]$/.test(node.tag) || node.role || node.tag === 'button') {
    return clean(node.text);
  }
  return '';
}

export function inferComponentIdentity(root, nodes, fallback) {
  const heading = nodes.find((node) => /^h[1-6]$/.test(node.tag) && clean(node.text));
  const control = nodes.find((node) =>
    clean(node.ariaLabel || node.attrs?.['aria-label'] || node.attrs?.placeholder),
  );
  const parents = new Set(nodes.map((node) => node.parentPath).filter(Boolean));
  const leaf = nodes.find((node) =>
    !parents.has(node.path) && clean(node.text) && !['svg', 'path'].includes(node.tag),
  );
  const classHint = String(root?.attrs?.class || '').split(/\s+/)
    .find((value) => !isHashClass(value));
  const choices = [
    ['root-aria', root?.ariaLabel || root?.attrs?.['aria-label']],
    ['heading', heading?.text],
    ['descendant-control', control?.ariaLabel || control?.attrs?.['aria-label']],
    ['leaf-text', leaf?.text],
    ['id', root?.attrs?.id],
    ['role', root?.role],
    ['landmark', ['header', 'nav', 'main', 'section', 'article', 'aside', 'footer', 'form', 'dialog']
      .includes(root?.tag) ? root.tag : ''],
    ['fallback', fallback],
  ];
  const selected = choices.find(([, value]) => clean(value));
  return {
    label: clean(selected?.[1], 100),
    labelSource: selected?.[0],
    tag: root?.tag,
    role: root?.role,
    ariaLabel: clean(root?.ariaLabel || root?.attrs?.['aria-label']) || undefined,
    heading: clean(heading?.text) || undefined,
    id: root?.attrs?.id,
    classHint,
  };
}

export function dedupeComponentCandidates(candidates) {
  const seen = new Set();
  return candidates.filter(({ node }) => {
    if (seen.has(node.fingerprint)) return false;
    seen.add(node.fingerprint);
    return true;
  });
}

function compactNodeIdentity(node, rootPath, childPaths) {
  const attrs = pick(node.attrs, ATTR_KEYS);
  const label = nodeLabel(node, childPaths);
  return {
    path: relativePath(node.path, rootPath),
    parent: node.parentPath && within(node.parentPath, rootPath)
      ? relativePath(node.parentPath, rootPath)
      : null,
    tag: node.tag,
    role: node.role || attrs.role || undefined,
    label: label || undefined,
    attrs: Object.keys(attrs).length ? attrs : undefined,
  };
}

function compactNodeGeometry(node, rootPath, parentStyle) {
  const style = pick(node.style, STYLE_KEYS);
  const styleDelta = Object.fromEntries(
    Object.entries(style).filter(([key, value]) => parentStyle?.[key] !== value),
  );
  return {
    path: relativePath(node.path, rootPath),
    rect: node.rect,
    style: styleDelta,
  };
}

function compactCapture(capture, rootPath) {
  const nodes = capture.nodes.filter((node) => within(node.path, rootPath));
  const childPaths = new Set(nodes.map((node) => node.parentPath).filter(Boolean));
  const byPath = new Map(nodes.map((node) => [node.path, node]));
  const selected = nodes.slice(0, 48);
  return {
    structure: selected.map((node) =>
      compactNodeIdentity(node, rootPath, childPaths)),
    viewport: capture.viewport,
    nodes: selected.map((node) => compactNodeGeometry(
      node,
      rootPath,
      byPath.get(node.parentPath)?.style,
    )),
    truncatedNodeCount: Math.max(0, nodes.length - 48),
    controls: (capture.behaviors || []).map((behavior) => ({
      path: relativePath(behavior.path, rootPath),
      tag: behavior.tag,
      role: behavior.role,
      label: clean(behavior.label),
      href: behavior.href,
      state: pick(behavior, [
        'ariaExpanded', 'ariaPressed', 'ariaSelected', 'ariaHaspopup',
      ]),
      events: [...new Set((behavior.listeners || []).map((listener) => listener.type))],
    })),
    assets: (capture.exactAssets || []).map((asset) => ({
      path: relativePath(asset.path, rootPath),
      type: asset.type,
      src: asset.currentSrc || asset.src || asset.file,
      width: asset.naturalWidth,
      height: asset.naturalHeight,
    })),
  };
}

export function buildAgentComponent(component) {
  const captures = component.captures.map((capture) =>
    compactCapture(capture, component.candidate.representativePath));
  return {
    schemaVersion: 1,
    purpose: 'Readable native-component build evidence. Never ship this artifact.',
    id: component.id,
    identity: component.identity,
    instances: {
      count: component.candidate.occurrencePaths.length,
      samplePaths: component.candidate.occurrencePaths.slice(0, 3),
    },
    reasons: component.candidate.reasons,
    structure: {
      nodes: captures[0]?.structure || [],
      truncatedNodeCount: captures[0]?.truncatedNodeCount || 0,
    },
    viewports: captures.map(({ structure: _structure, ...capture }) => capture),
    responsive: component.responsive,
    animationImplementations: component.animationImplementations,
  };
}

export function isUsefulAgentComponent(component) {
  const count = component.captures[0]?.nodes.length || 0;
  return Boolean(component.identity.label) &&
    component.identity.labelSource !== 'fallback' &&
    count >= 2 &&
    count <= 120;
}
