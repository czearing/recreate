const normalizedAttributes = (node) => (node.attrs || [])
  .filter(({ name }) => !name.startsWith('data-rdwebrtc'))
  .map(({ name }) => name)
  .sort()
  .join('|');

export function structuralSignature(node) {
  if (node.nodeName === '#text') return '#text';
  if (!node.tagName) return node.nodeName;
  if (node.tagName === 'svg') return `svg[${normalizedAttributes(node)}](*)`;
  return `${node.tagName}[${normalizedAttributes(node)}](` +
    `${(node.childNodes || []).map(structuralSignature).join(',')})`;
}

function nodeCount(node) {
  return 1 + (node.childNodes || []).reduce((total, child) => total + nodeCount(child), 0);
}

function descendants(node) {
  return [node, ...(node.childNodes || []).flatMap(descendants)];
}

function valueAt(node, path) {
  return path.reduce((current, index) => current?.childNodes?.[index], node);
}

function propName(base, used) {
  const clean = String(base || 'value').replace(/[^a-z0-9]+/gi, ' ').trim()
    .split(/\s+/).map((word, index) =>
      index ? word[0]?.toUpperCase() + word.slice(1) : word.toLowerCase()).join('');
  const candidate = /^[a-z]/.test(clean) ? clean : `value${clean}`;
  const root = ['class', 'default', 'delete', 'function', 'new', 'return', 'var']
    .includes(candidate) ? `${candidate}Value` : candidate;
  const count = used.get(root) || 0;
  used.set(root, count + 1);
  return count ? `${root}${count + 1}` : root;
}

function collectFields(nodes, path = [], fields = [], used = new Map()) {
  const first = valueAt(nodes[0], path);
  if (!first) return fields;
  if (first.tagName === 'svg') {
    fields.push({ kind: 'node', path, name: propName('icon', used), values: nodes
      .map((node) => valueAt(node, path)) });
    return fields;
  }
  if (first.nodeName === '#text') {
    const values = nodes.map((node) => valueAt(node, path)?.value || '');
    if (new Set(values).size > 1) {
      fields.push({ kind: 'text', path, name: propName('text', used), values });
    }
    return fields;
  }
  for (const { name } of first.attrs || []) {
    const values = nodes.map((node) =>
      valueAt(node, path)?.attrs?.find((attr) => attr.name === name)?.value || '');
    if (new Set(values).size > 1) {
      fields.push({ kind: 'attr', attr: name, path, name: propName(name, used), values });
    }
  }
  for (let index = 0; index < (first.childNodes || []).length; index += 1) {
    collectFields(nodes, [...path, index], fields, used);
  }
  return fields;
}

export function findRepeatedComponents(root, { minNodes = 4, maxNodes = 120 } = {}) {
  const bySignature = new Map();
  const visit = (node) => {
    if (node.tagName) {
      const count = nodeCount(node);
      if (count >= minNodes && count <= maxNodes) {
        const signature = structuralSignature(node);
        const group = bySignature.get(signature) || [];
        group.push(node);
        bySignature.set(signature, group);
      }
    }
    for (const child of node.childNodes || []) visit(child);
  };
  visit(root);
  const covered = new WeakSet();
  const groups = [...bySignature.values()]
    .filter((nodes) => nodes.length > 1)
    .sort((left, right) => nodeCount(right[0]) - nodeCount(left[0]));
  const byNode = new WeakMap();
  for (const nodes of groups) {
    const available = nodes.filter((node) => !covered.has(node));
    if (available.length < 2) continue;
    const fields = collectFields(available);
    if (!fields.length) continue;
    const group = { nodes: available, fields, name: '' };
    for (const node of available) {
      byNode.set(node, group);
      for (const descendant of descendants(node)) covered.add(descendant);
    }
  }
  return byNode;
}

export function fieldMaps(group) {
  const text = new Map();
  const attrs = new Map();
  const nodes = new Map();
  for (const field of group?.fields || []) {
    const key = field.path.join('.');
    if (field.kind === 'text') text.set(key, field.name);
    else if (field.kind === 'node') nodes.set(key, field.name);
    else {
      const values = attrs.get(key) || new Map();
      values.set(field.attr, field.name);
      attrs.set(key, values);
    }
  }
  return { text, attrs, nodes };
}
