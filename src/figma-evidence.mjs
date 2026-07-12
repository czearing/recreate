import fs from 'node:fs';
import path from 'node:path';
import { compactFigmaNode, figmaGuid as guid } from './figma-node.mjs';
import { writeFigmaSection } from './figma-sections.mjs';
import { resolveFigmaGeometry } from './figma-vectors.mjs';
import { writeFigmaComponents } from './figma-components.mjs';
import { writeFigmaInteractions } from './figma-interactions.mjs';
import { writeFigmaStyles } from './figma-styles.mjs';
import { createFigmaValueStore } from './figma-values.mjs';
import { resolveVectorNetwork } from './figma-vector-network.mjs';

const slug = (value) =>
  String(value || 'page')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 60) || 'page';

export function writeFigmaEvidence({ outDir, source, decoded, byteLength, profile }) {
  const evidenceDir = path.join(outDir, 'evidence', 'figma');
  fs.mkdirSync(evidenceDir, { recursive: true });
  const nodes = decoded.message.nodeChanges;
  const blobs = (decoded.message.blobs || []).map((blob) =>
    blob.bytes instanceof Uint8Array
      ? blob.bytes
      : Uint8Array.from(Object.values(blob.bytes || {}))
  );
  const valueStore = createFigmaValueStore(evidenceDir);
  const { reference } = valueStore;
  const geometryStats = { total: 0, decoded: 0, errors: 0 };
  const vectorStats = { total: 0, decoded: 0, errors: 0, paths: 0 };
  const geometry = (paths) => {
    const resolved = resolveFigmaGeometry(paths, blobs);
    for (const item of resolved || []) {
      geometryStats.total += 1;
      if (item.d) geometryStats.decoded += 1;
      if (item.error) geometryStats.errors += 1;
    }
    return resolved;
  };
  const vectorNetwork = (node) => {
    const resolved = resolveVectorNetwork(node, blobs);
    if (!resolved) return undefined;
    vectorStats.total += 1;
    vectorStats.paths += resolved.paths.length;
    if (resolved.error) vectorStats.errors += 1;
    else vectorStats.decoded += 1;
    return resolved;
  };
  const compactCache = new WeakMap();
  const compact = (node) => {
    let result = compactCache.get(node);
    if (!result) {
      result = compactFigmaNode(node, reference, geometry, vectorNetwork);
      compactCache.set(node, result);
    }
    return result;
  };
  const children = new Map();
  for (const node of nodes) {
    const parentId = guid(node.parentIndex?.guid);
    if (!children.has(parentId)) children.set(parentId, []);
    children.get(parentId).push(node);
  }
  const pageForNode = new Map();
  const pages = nodes.filter((node) => node.type === 'CANVAS');
  for (const page of pages) {
    const pageId = guid(page.guid);
    const queue = [...(children.get(pageId) || [])];
    for (let cursor = 0; cursor < queue.length; cursor += 1) {
      const node = queue[cursor];
      const id = guid(node.guid);
      pageForNode.set(id, pageId);
      queue.push(...(children.get(id) || []));
    }
  }
  const pageIndex = [];
  for (const [index, page] of pages.entries()) {
    const pageId = guid(page.guid);
    const pageNodeCount = nodes.filter(
      (node) => pageForNode.get(guid(node.guid)) === pageId,
    ).length;
    if (profile !== 'full' && page.visible === false) {
      pageIndex.push({
        id: pageId,
        name: page.name,
        visible: false,
        nodeCount: pageNodeCount,
        evidence: null,
        omittedFromImplementation: true,
      });
      continue;
    }
    const filename = `${String(index).padStart(2, '0')}-${slug(page.name)}.json`;
    const sectionDir = path.join(
      evidenceDir,
      `${String(index).padStart(2, '0')}-${slug(page.name)}`,
    );
    fs.mkdirSync(sectionDir, { recursive: true });
    const relativeSectionDir =
      `evidence/figma/${String(index).padStart(2, '0')}-${slug(page.name)}`;
    const sections = (children.get(pageId) || []).map((root, sectionIndex) =>
      writeFigmaSection({
        root,
        index: sectionIndex,
        directory: sectionDir,
        relativeDirectory: relativeSectionDir,
        children,
        compact,
      }),
    );
    fs.writeFileSync(
      path.join(evidenceDir, filename),
      JSON.stringify({
        id: pageId,
        name: page.name,
        visible: page.visible,
        sections,
      }),
    );
    pageIndex.push({
      id: pageId,
      name: page.name,
      visible: page.visible,
      nodeCount: pageNodeCount,
      evidence: `evidence/figma/${filename}`,
    });
  }
  const variables = nodes
    .filter((node) => node.type === 'VARIABLE' || node.type === 'VARIABLE_SET')
    .map(compact);
  const componentIndex = writeFigmaComponents({
    outDir,
    nodes,
    pageForNode,
    reference,
    compact,
    children,
    visiblePageIds: new Set(
      pages.filter((page) => page.visible !== false).map((page) => guid(page.guid)),
    ),
  });
  const interactionIndex = writeFigmaInteractions({
    outDir,
    nodes,
    pageForNode,
    reference,
  });
  const styleIndex = writeFigmaStyles({
    outDir,
    nodes,
    pageForNode,
    compact,
  });
  const fontUsage = new Map();
  for (const node of nodes) {
    if (!node.fontName) continue;
    const key = JSON.stringify(node.fontName);
    const entry = fontUsage.get(key) || {
      ...node.fontName,
      usageCount: 0,
      localized: false,
    };
    entry.usageCount += 1;
    fontUsage.set(key, entry);
  }
  const fonts = [...fontUsage.values()].sort((left, right) =>
    `${left.family} ${left.style}`.localeCompare(`${right.family} ${right.style}`)
  );
  fs.writeFileSync(
    path.join(evidenceDir, 'variables.json'),
    JSON.stringify({ variables }),
  );
  const valueIndex = valueStore.write();
  const index = {
    sourceType: source.kind,
    fileId: source.fileId,
    version: decoded.version,
    byteLength,
    profile,
    schemaDefinitionCount: decoded.schemaDefinitionCount,
    nodeCount: nodes.length,
    componentCount: nodes.filter((node) => node.type === 'SYMBOL').length,
    components: componentIndex,
    interactions: interactionIndex,
    styles: styleIndex,
    instanceCount: nodes.filter((node) => node.type === 'INSTANCE').length,
    variableCount: variables.length,
    fontCount: fonts.length,
    fonts,
    geometry: geometryStats,
    vectorNetworks: vectorStats,
    pages: pageIndex,
    variables: 'evidence/figma/variables.json',
    values: valueIndex,
  };
  fs.writeFileSync(path.join(outDir, 'figma.json'), JSON.stringify(index, null, 2));
  return index;
}
