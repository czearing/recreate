import { styleDelta } from './agent-style.mjs';

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
  if (node.tag === '#text') return clean(node.text);
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

function compactNodeIdentity(node, key, rootPath, childPaths) {
  const attrs = pick(node.attrs, ATTR_KEYS);
  const label = nodeLabel(node, childPaths);
  return {
    path: key,
    parent: node.parentPath && within(node.parentPath, rootPath)
      ? relativePath(node.parentPath, rootPath)
      : null,
    tag: node.tag,
    role: node.role || attrs.role || undefined,
    label: label || undefined,
    attrs: Object.keys(attrs).length ? attrs : undefined,
  };
}

function compactNodeGeometry(node, key, parentStyle) {
  return {
    path: key,
    rect: node.rect,
    style: styleDelta(node.style, parentStyle, node.tag === '#text'),
  };
}

function compactCapture(capture, rootPath, excludeRoots) {
  const included = (pathValue) =>
    within(pathValue, rootPath) &&
    !excludeRoots.some((excluded) => within(pathValue, excluded));
  const nodes = capture.nodes.filter((node) => included(node.path));
  const childPaths = new Set(nodes.map((node) => node.parentPath).filter(Boolean));
  const byPath = new Map(nodes.map((node) => [node.path, node]));
  const pathCounts = new Map();
  const selected = nodes.map((node) => {
    const base = relativePath(node.path, rootPath);
    const count = pathCounts.get(base) || 0;
    pathCounts.set(base, count + 1);
    return {
      node,
      key: node.tag === '#text' ? `${base}::text(${count})` : base,
    };
  });
  return {
    structure: selected.map(({ node, key }) =>
      compactNodeIdentity(node, key, rootPath, childPaths)),
    viewport: capture.viewport,
    nodes: selected.map(({ node, key }) => compactNodeGeometry(
      node,
      key,
      byPath.get(node.parentPath)?.style,
    )),
    truncatedNodeCount: 0,
    controls: (capture.behaviors || []).filter((behavior) => included(behavior.path)).map((behavior) => ({
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
    assets: (capture.exactAssets || []).filter((asset) => included(asset.path)).map((asset) => ({
      path: relativePath(asset.path, rootPath),
      type: asset.type,
      src: asset.currentSrc || asset.src || asset.file,
      width: asset.naturalWidth,
      height: asset.naturalHeight,
    })),
  };
}

export function buildAgentComponent(component, { excludeRoots = [] } = {}) {
  const captures = component.captures.map((capture) =>
    compactCapture(capture, component.candidate.representativePath, excludeRoots));
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
    nativeHints: captures[0]?.controls.map((control) => ({
      path: control.path,
      label: control.label,
      primitive: control.role === 'menuitem' ? 'Fluent MenuItem' :
        control.role === 'menu' ? 'Fluent Menu' :
        control.tag === 'button' ? 'Fluent Button' : 'destination-native control',
    })) || [],
  };
}

export function isUsefulAgentComponent(component) {
  const count = component.captures[0]?.nodes.length || 0;
  return Boolean(component.identity.label) &&
    component.identity.labelSource !== 'fallback' &&
    count >= 2 &&
    count <= 120;
}
