import fs from 'node:fs';
import path from 'node:path';
import { figmaGuid as guid } from './figma-node.mjs';

const styleFields = {
  fill: 'styleIdForFill',
  stroke: 'styleIdForStrokeFill',
  text: 'styleIdForText',
  effect: 'styleIdForEffect',
  grid: 'styleIdForGrid',
};

export function writeFigmaStyles({
  outDir,
  nodes,
  pageForNode,
  compact,
}) {
  const nodesById = new Map(nodes.map((node) => [guid(node.guid), node]));
  const usage = new Map();
  for (const node of nodes) {
    for (const [kind, field] of Object.entries(styleFields)) {
      const id = guid(node[field]?.guid);
      if (!id || id === '4294967295:4294967295') continue;
      const entry = usage.get(id) || { count: 0, kinds: new Set() };
      entry.count += 1;
      entry.kinds.add(kind);
      usage.set(id, entry);
    }
  }
  const styles = {};
  const missing = [];
  for (const [id, used] of usage) {
    const node = nodesById.get(id);
    if (!node) {
      missing.push(id);
      continue;
    }
    styles[id] = {
      id,
      name: node.name,
      type: node.type,
      pageId: pageForNode.get(id),
      usageCount: used.count,
      kinds: [...used.kinds],
      definition: compact(node),
    };
  }
  const file = 'evidence/figma/styles.json';
  fs.writeFileSync(
    path.join(outDir, file),
    JSON.stringify({ styles, missing }),
  );
  return {
    count: Object.keys(styles).length,
    referenceCount: [...usage.values()]
      .reduce((total, entry) => total + entry.count, 0),
    missingCount: missing.length,
    file,
  };
}
