import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { figmaGuid as guid } from './figma-node.mjs';

export function writeFigmaComponents({
  outDir,
  nodes,
  pageForNode,
  reference,
  compact,
  children,
}) {
  const usage = new Map();
  for (const node of nodes) {
    if (node.type !== 'INSTANCE') continue;
    const symbolId = guid(node.symbolData?.symbolID);
    if (!symbolId) continue;
    const entry = usage.get(symbolId) || {
      count: 0,
      pageIds: new Set(),
      examples: [],
    };
    entry.count += 1;
    const pageId = pageForNode.get(guid(node.guid));
    if (pageId) entry.pageIds.add(pageId);
    if (entry.examples.length < 10) {
      entry.examples.push({
        id: guid(node.guid),
        name: node.name,
        pageId,
        properties: reference(node.componentPropAssignments),
      });
    }
    usage.set(symbolId, entry);
  }
  const components = nodes
    .filter((node) => node.type === 'SYMBOL')
    .map((node) => {
      const id = guid(node.guid);
      const instances = usage.get(id);
      const queue = [...(children.get(id) || [])];
      const descendants = [];
      for (let cursor = 0; cursor < queue.length; cursor += 1) {
        const child = queue[cursor];
        descendants.push(compact(child));
        queue.push(...(children.get(guid(child.guid)) || []));
      }
      return {
        id,
        name: node.name,
        parentId: guid(node.parentIndex?.guid),
        pageId: pageForNode.get(id),
        size: node.size,
        publishable: node.isSymbolPublishable,
        description: node.symbolDescription,
        variantProperties: reference(node.variantPropSpecs),
        propertyDefinitions: reference(node.componentPropDefs),
        master: compact(node),
        masterChildren: reference(descendants),
        instanceUsage: instances
          ? {
              count: instances.count,
              pageIds: [...instances.pageIds],
              examples: instances.examples,
            }
          : { count: 0, pageIds: [], examples: [] },
      };
    });
  const componentDir = path.join(outDir, 'evidence', 'figma', 'components');
  fs.mkdirSync(componentDir, { recursive: true });
  const shards = {};
  const search = components.map((component) => {
    const prefix = createHash('sha256')
      .update(component.id)
      .digest('hex')[0];
    shards[prefix] ||= {};
    shards[prefix][component.id] = component;
    return {
      id: component.id,
      name: component.name,
      parentId: component.parentId,
      pageId: component.pageId,
      instanceCount: component.instanceUsage.count,
      detailPrefix: prefix,
    };
  });
  const searchFile = 'evidence/figma/component-search.json';
  fs.writeFileSync(
    path.join(outDir, searchFile),
    JSON.stringify({ components: search }),
  );
  const shardFiles = [];
  for (const [prefix, shardComponents] of Object.entries(shards)) {
    const file = `evidence/figma/components/${prefix}.json`;
    fs.writeFileSync(
      path.join(outDir, file),
      JSON.stringify({ components: shardComponents }),
    );
    shardFiles.push(file);
  }
  return {
    count: components.length,
    usedCount: components.filter((component) => component.instanceUsage.count).length,
    search: searchFile,
    details: {
      pattern: 'evidence/figma/components/<detail-prefix>.json',
      shards: shardFiles.sort(),
    },
  };
}
