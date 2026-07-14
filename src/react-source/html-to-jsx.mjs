import { parse } from 'parse5';
import { jsxText, renderAttributes, VOID_TAGS } from './jsx-format.mjs';
import { fieldMaps, findRepeatedComponents } from './repeated-components.mjs';

const cleanName = (value) => {
  const words = String(value || '').replace(/[^a-z0-9]+/gi, ' ').trim().split(/\s+/);
  const name = words.map((word) => word[0]?.toUpperCase() + word.slice(1)).join('');
  return /^[A-Z]/.test(name) ? name.slice(0, 48) : `Section${name || 'Block'}`;
};

function bodyNode(document) {
  const html = document.childNodes.find((node) => node.tagName === 'html');
  return html?.childNodes.find((node) => node.tagName === 'body');
}

function textFor(node, limit = 48) {
  if (node.nodeName === '#text') return String(node.value || '').trim().slice(0, limit);
  for (const child of node.childNodes || []) {
    const text = textFor(child, limit);
    if (text) return text;
  }
  return '';
}

function nodeCount(node) {
  return 1 + (node.childNodes || []).reduce((total, child) => total + nodeCount(child), 0);
}

function nodeSignature(node) {
  if (node.nodeName === '#text') return `#text:${node.value || ''}`;
  if (!node.tagName) return node.nodeName;
  const attrs = (node.attrs || [])
    .filter(({ name }) => !name.startsWith('data-rdwebrtc'))
    .map(({ name, value }) => `${name}=${value}`)
    .sort()
    .join('|');
  return `${node.tagName}[${attrs}](${(node.childNodes || []).map(nodeSignature).join(',')})`;
}

function isComponentBoundary(node) {
  const attrs = new Map((node.attrs || []).map(({ name, value }) => [name, value]));
  const elementChildren = (node.childNodes || []).filter((child) => child.tagName);
  return (
    elementChildren.length > 1 ||
    ['article', 'aside', 'button', 'dialog', 'footer', 'form', 'header', 'main', 'nav', 'section']
      .includes(node.tagName) ||
    attrs.has('aria-label') ||
    attrs.has('data-testid') ||
    attrs.has('role')
  );
}

export function generateReactComponents(html, { maxNodes = 20 } = {}) {
  const body = bodyNode(parse(html));
  if (!body) throw new Error('Captured HTML has no body.');
  const definitions = [];
  const usedNames = new Map();
  const componentBySignature = new Map();
  const repeatedByNode = findRepeatedComponents(body);

  const uniqueName = (node) => {
    const label = node.attrs?.find(({ name }) =>
      ['aria-label', 'title'].includes(name))?.value || textFor(node);
    const base = cleanName(label || node.tagName);
    const count = usedNames.get(base) || 0;
    usedNames.set(base, count + 1);
    return count ? `${base}${count + 1}` : base;
  };

  const repeatedInvocation = (group, node, depth, definition) => {
    if (!group.name) emitRepeated(group);
    const index = group.nodes.indexOf(node);
    const props = group.fields.map((field) => {
      if (field.kind !== 'node') {
        return `${field.name}=${JSON.stringify(field.values[index])}`;
      }
      const childName = emit(field.values[index]);
      definition.imports.push(childName);
      return `${field.name}={<${childName} />}`;
    })
      .join(' ');
    return `${'  '.repeat(depth)}<${group.name}${props ? ` ${props}` : ''} />`;
  };

  const emitRepeated = (group) => {
    if (group.name) return group.name;
    group.name = `${uniqueName(group.nodes[0])}Item`;
    const definition = {
      name: group.name,
      kind: group.nodes[0].tagName === 'svg' ? 'icon' : 'component',
      imports: [],
      props: group.fields.map(({ kind, name }) => ({
        name,
        type: kind === 'node' ? 'ReactNode' : 'string',
      })),
      jsx: '',
    };
    definitions.push(definition);
    definition.jsx = renderNode(group.nodes[0], 2, group.nodes[0], definition, {
      group,
      maps: fieldMaps(group),
    });
    return group.name;
  };

  const emit = (node, requestedName) => {
    const signature = nodeSignature(node);
    const existing = componentBySignature.get(signature);
    if (existing) return existing;
    const name = requestedName || uniqueName(node);
    componentBySignature.set(signature, name);
    const definition = {
      name,
      kind: node.tagName === 'svg' ? 'icon' : 'component',
      imports: [],
      props: [],
      jsx: '',
    };
    definitions.push(definition);
    definition.jsx = renderNode(node, 2, node, definition);
    return name;
  };

  const renderNode = (
    node,
    depth,
    root,
    definition,
    template,
    nodePath = [],
  ) => {
    const pathKey = nodePath.join('.');
    if (node.nodeName === '#text') {
      const prop = template?.maps.text.get(pathKey);
      return `${'  '.repeat(depth)}${prop ? `{${prop}}` : jsxText(node.value)}`;
    }
    if (node.nodeName === '#comment' || !node.tagName) return '';
    const nodeProp = template?.maps.nodes.get(pathKey);
    if (nodeProp) return `${'  '.repeat(depth)}{${nodeProp}}`;
    if (['script', 'style', 'link', 'meta', 'base', 'noscript'].includes(node.tagName)) return '';
    const repeated = repeatedByNode.get(node);
    if (node !== root && repeated && repeated !== template?.group) {
      const name = emitRepeated(repeated);
      definition.imports.push(name);
      return repeatedInvocation(repeated, node, depth, definition);
    }
    if (!template && node !== root && nodeCount(node) > maxNodes && isComponentBoundary(node)) {
      const childName = emit(node);
      definition.imports.push(childName);
      return `${'  '.repeat(depth)}<${childName} />`;
    }
    const attrs = renderAttributes(node.attrs, template?.maps.attrs.get(pathKey));
    const open = `<${node.tagName}${attrs ? ` ${attrs}` : ''}`;
    if (VOID_TAGS.has(node.tagName)) return `${'  '.repeat(depth)}${open} />`;
    const children = (node.childNodes || [])
      .map((child, index) =>
        renderNode(child, depth + 1, root, definition, template, [...nodePath, index]))
      .filter(Boolean);
    if (!children.length) return `${'  '.repeat(depth)}${open} />`;
    return [
      `${'  '.repeat(depth)}${open}>`,
      ...children,
      `${'  '.repeat(depth)}</${node.tagName}>`,
    ].join('\n');
  };

  const roots = (body.childNodes || []).filter((node) =>
    node.tagName || (node.nodeName === '#text' && String(node.value || '').trim()));
  const appImports = [];
  const appChildren = roots.map((node) => {
    const name = emit(node, uniqueName(node));
    appImports.push(name);
    return `      <${name} />`;
  });
  return { definitions, appImports, appChildren };
}
