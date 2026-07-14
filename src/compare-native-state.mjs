#!/usr/bin/env node
import fs from 'node:fs';

const clean = (value) => String(value || '').replace(/\s+/g, ' ').trim();
const TEXT_PAINT_PROPERTIES = [
  'color',
  'fontFamily',
  'fontSize',
  'fontWeight',
  'lineHeight',
  'letterSpacing',
  'textAlign',
  'opacity',
  'transform',
];
const CONTAINER_PAINT_PROPERTIES = [
  'backgroundColor',
  'backgroundImage',
  'border',
  'borderRadius',
  'boxShadow',
  'opacity',
  'transform',
];
const identities = (node, { childCounts, nodesByPath }) => {
  const result = [];
  const attrs = node.attrs || {};
  const label = clean(node.ariaLabel || attrs['aria-label']);
  if (label) result.push(`aria|${label}`);
  const testId = clean(attrs['data-testid']);
  if (testId) result.push(`testid|${testId}`);
  const text = clean(node.text);
  const interactiveNode = /^(?:button|textarea|input|select)$/.test(node.tag);
  if (!label && interactiveNode && text) {
    result.push(`control|${text}`);
  }
  if (
    !childCounts.get(node.path) &&
    text &&
    text.length <= 300 &&
    (!interactiveNode || Boolean(label))
  ) {
    let parentPath = node.parentPath;
    const visited = new Set();
    while (parentPath && !visited.has(parentPath)) {
      visited.add(parentPath);
      const parent = nodesByPath.get(parentPath);
      if (!parent) break;
      const parentText = clean(parent.text);
      if (
        /^(?:button|textarea|input|select)$/.test(parent.tag) &&
        parentText === text
      ) {
        return result;
      }
      parentPath = parent.parentPath;
    }
    if (text !== label) result.push(`text|${text}`);
  }
  return result;
};
const delta = (left, right) => Math.max(
  ...['x', 'y', 'width', 'height'].map((key) =>
    Math.abs((left.rect?.[key] || 0) - (right.rect?.[key] || 0))),
);
const isRendered = (node) =>
  node.rect?.width > 0 && node.rect?.height > 0 &&
  node.style?.display !== 'none' &&
  node.style?.visibility !== 'hidden';
const isPainted = (node) =>
  isRendered(node) && Number.parseFloat(node.style?.opacity ?? '1') > 0;
const isComparable = (node) => {
  if (!isRendered(node)) return false;
  const interactive =
    /^(?:button|input|textarea|select|a)$/.test(node.tag) ||
    ['button', 'link', 'menuitem', 'option', 'switch', 'tab'].includes(
      String(node.role || node.attrs?.role || '').toLowerCase(),
    );
  return !interactive || (node.rect.width >= 8 && node.rect.height >= 8);
};

function group(nodes) {
  const childCounts = new Map();
  const nodesByPath = new Map(nodes.map((node) => [node.path, node]));
  for (const node of nodes) {
    if (node.parentPath) {
      childCounts.set(node.parentPath, (childCounts.get(node.parentPath) || 0) + 1);
    }
  }
  const groups = new Map();
  for (const node of nodes.filter(isComparable)) {
    node.__siteSpecHasChildren = Boolean(childCounts.get(node.path));
    for (const key of identities(node, { childCounts, nodesByPath })) {
      const values = groups.get(key) || [];
      values.push(node);
      groups.set(key, values);
    }
  }
  return groups;
}

const transparent = (value) =>
  !value ||
  value === 'transparent' ||
  /^rgba?\([^)]*,\s*0(?:\.0+)?\)$/.test(value);
const hasOwnPaint = (node) => {
  const style = node.style || {};
  return (
    !transparent(style.backgroundColor) ||
    (style.backgroundImage && style.backgroundImage !== 'none') ||
    (style.border && !style.border.startsWith('0px')) ||
    (style.boxShadow && style.boxShadow !== 'none')
  );
};
const canonicalPaint = (property, value, node) => {
  if (property === 'textAlign' && value === 'start') return 'left';
  if (property === 'borderRadius') {
    const radius = Number.parseFloat(value);
    if (
      value === '50%' ||
      radius >= Math.min(node.rect?.width || Infinity, node.rect?.height || Infinity) / 2
    ) return 'circular';
  }
  if ((property === 'backgroundColor' || property === 'border') && transparent(value)) {
    return 'transparent';
  }
  if (property === 'border') {
    const match = value.match(/^([\d.]+)px\s+(\S+)\s+(.+)$/);
    if (match && Number.parseFloat(match[1]) <= 1) {
      return `thin ${match[2]} ${match[3]}`;
    }
  }
  return value;
};

function comparePaint(reference, candidate) {
  const ownPaint = hasOwnPaint(reference);
  if (reference.__siteSpecHasChildren && !ownPaint) return [];
  const properties = ownPaint ? CONTAINER_PAINT_PROPERTIES : TEXT_PAINT_PROPERTIES;
  return properties.flatMap((property) => {
    const expected = canonicalPaint(
      property,
      String(reference.style?.[property] ?? ''),
      reference,
    );
    const actual = canonicalPaint(
      property,
      String(candidate.style?.[property] ?? ''),
      candidate,
    );
    return expected === actual ? [] : [{ property, reference: expected, candidate: actual }];
  });
}

function matchGroup(expected, actual) {
  if (!expected.length || !actual.length) return new Map();
  const expectedIsSmaller = expected.length <= actual.length;
  const smaller = expectedIsSmaller ? expected : actual;
  const larger = expectedIsSmaller ? actual : expected;
  if (larger.length > 8) {
    const pairs = [];
    for (const [expectedIndex, referenceNode] of expected.entries()) {
      for (const [candidateIndex, candidateNode] of actual.entries()) {
        pairs.push({
          expectedIndex,
          candidateIndex,
          cost: delta(referenceNode, candidateNode),
        });
      }
    }
    const matchedExpected = new Set();
    const matchedActual = new Set();
    const result = new Map();
    for (const pair of pairs.sort((left, right) => left.cost - right.cost)) {
      if (matchedExpected.has(pair.expectedIndex) || matchedActual.has(pair.candidateIndex)) continue;
      matchedExpected.add(pair.expectedIndex);
      matchedActual.add(pair.candidateIndex);
      result.set(pair.expectedIndex, pair.candidateIndex);
    }
    return result;
  }
  const memo = new Map();
  const visit = (index, used) => {
    if (index === smaller.length) return { cost: 0, choices: [] };
    const key = `${index}:${used}`;
    if (memo.has(key)) return memo.get(key);
    let best;
    for (let largerIndex = 0; largerIndex < larger.length; largerIndex += 1) {
      const bit = 1 << largerIndex;
      if (used & bit) continue;
      const smallerNode = smaller[index];
      const largerNode = larger[largerIndex];
      const tail = visit(index + 1, used | bit);
      const cost = delta(smallerNode, largerNode) + tail.cost;
      if (!best || cost < best.cost) {
        best = { cost, choices: [largerIndex, ...tail.choices] };
      }
    }
    memo.set(key, best);
    return best;
  };
  const result = new Map();
  for (const [smallerIndex, largerIndex] of visit(0, 0).choices.entries()) {
    const expectedIndex = expectedIsSmaller ? smallerIndex : largerIndex;
    const candidateIndex = expectedIsSmaller ? largerIndex : smallerIndex;
    result.set(expectedIndex, candidateIndex);
  }
  return result;
}

export function compareNativeState(reference, candidate) {
  const referenceGroups = group(reference.nodes || []);
  const candidateGroups = group(candidate.nodes || []);
  const rows = [];
  for (const [key, expected] of referenceGroups) {
    const actual = candidateGroups.get(key) || [];
    const matches = matchGroup(expected, actual);
    for (const [expectedIndex, referenceNode] of expected.entries()) {
      const candidateIndex = matches.get(expectedIndex);
      const candidateNode = candidateIndex === undefined ? undefined : actual[candidateIndex];
      const value = candidateNode ? delta(referenceNode, candidateNode) : undefined;
      const paintCompared = Boolean(candidateNode) &&
        (!referenceNode.__siteSpecHasChildren || hasOwnPaint(referenceNode));
      const paintDiffs = candidateNode ? comparePaint(referenceNode, candidateNode) : [];
      rows.push({
        identity: key,
        matched: Boolean(candidateNode),
        painted: isPainted(referenceNode),
        paintCompared,
        deltaPx: value,
        paintDiffs,
        reference: referenceNode.rect,
        candidate: candidateNode?.rect,
      });
    }
  }
  const paintedRows = rows.filter((row) => row.painted);
  const matched = rows.filter((row) => row.matched);
  const paintedMatched = paintedRows.filter((row) => row.matched);
  const paintComparedRows = paintedMatched.filter((row) => row.paintCompared);
  const paintRows = paintComparedRows.filter((row) => row.paintDiffs.length);
  const paintProperties = {};
  for (const row of paintRows) {
    for (const difference of row.paintDiffs) {
      paintProperties[difference.property] = (paintProperties[difference.property] || 0) + 1;
    }
  }
  return {
    required: rows.length,
    matched: matched.length,
    missing: rows.filter((row) => !row.matched).map((row) => row.identity),
    maxDeltaPx: matched.length ? Math.max(...matched.map((row) => row.deltaPx)) : null,
    painted: {
      required: paintedRows.length,
      matched: paintedMatched.length,
      maxDeltaPx: paintedMatched.length
        ? Math.max(...paintedMatched.map((row) => row.deltaPx))
        : null,
    },
    paint: {
      compared: paintComparedRows.length,
      mismatched: paintRows.length,
      properties: paintProperties,
      worst: paintRows
        .sort((left, right) => right.paintDiffs.length - left.paintDiffs.length)
        .slice(0, 20)
        .map(({ identity: key, paintDiffs }) => ({ identity: key, differences: paintDiffs })),
    },
    worst: matched.sort((left, right) => right.deltaPx - left.deltaPx).slice(0, 20),
  };
}

if (import.meta.main) {
  const args = Object.fromEntries(process.argv.slice(2).map((value, index, all) => [
    value.replace(/^--/, ''),
    all[index + 1] && !all[index + 1].startsWith('--') ? all[index + 1] : true,
  ]));
  if (!args.reference || !args.candidate) {
    throw new Error('Pass --reference <state.json> --candidate <state.json>.');
  }
  const result = compareNativeState(
    JSON.parse(fs.readFileSync(String(args.reference), 'utf8')),
    JSON.parse(fs.readFileSync(String(args.candidate), 'utf8')),
  );
  console.log(JSON.stringify(result, null, 2));
  if (result.missing.length) process.exitCode = 1;
}
