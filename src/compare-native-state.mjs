#!/usr/bin/env node
import fs from 'node:fs';

const clean = (value) => String(value || '').replace(/\s+/g, ' ').trim();
const identity = (node) => {
  const attrs = node.attrs || {};
  const label = node.ariaLabel || attrs['aria-label'] || attrs['data-testid'] ||
    (/^(?:button|textarea|h[1-6])$/.test(node.tag) ? node.text : '');
  return clean(label) ? `${node.tag}|${clean(label)}` : '';
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

function group(nodes) {
  const groups = new Map();
  for (const node of nodes.filter((item) => isRendered(item) && identity(item))) {
    const key = identity(node);
    const values = groups.get(key) || [];
    values.push(node);
    groups.set(key, values);
  }
  return groups;
}

export function compareNativeState(reference, candidate) {
  const referenceGroups = group(reference.nodes || []);
  const candidateGroups = group(candidate.nodes || []);
  const rows = [];
  for (const [key, expected] of referenceGroups) {
    const actual = candidateGroups.get(key) || [];
    const used = new Set();
    for (const referenceNode of expected) {
      let best;
      for (const [index, candidateNode] of actual.entries()) {
        if (used.has(index)) continue;
        const value = delta(referenceNode, candidateNode);
        if (!best || value < best.delta) best = { index, node: candidateNode, delta: value };
      }
      if (best) used.add(best.index);
      rows.push({
        identity: key,
        matched: Boolean(best),
        painted: isPainted(referenceNode),
        deltaPx: best?.delta,
        reference: referenceNode.rect,
        candidate: best?.node.rect,
      });
    }
  }
  const paintedRows = rows.filter((row) => row.painted);
  const matched = rows.filter((row) => row.matched);
  const paintedMatched = paintedRows.filter((row) => row.matched);
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
