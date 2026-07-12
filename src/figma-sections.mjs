import fs from 'node:fs';
import path from 'node:path';

const slug = (value) =>
  String(value || 'section')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 60) || 'section';

function collect(root, children, compact) {
  const queue = [root];
  const nodes = [];
  for (let cursor = 0; cursor < queue.length; cursor += 1) {
    const node = queue[cursor];
    const compacted = compact(node);
    nodes.push(compacted);
    queue.push(...(children.get(compacted.id) || []));
  }
  return nodes;
}

export function writeFigmaSection({
  root,
  index,
  directory,
  relativeDirectory,
  children,
  compact,
  depth = 0,
}) {
  const id = compact(root).id;
  const nodes = collect(root, children, compact);
  const directChildren = children.get(id) || [];
  const basename = `${String(index).padStart(2, '0')}-${slug(root.name)}`;
  const filename = `${basename}.json`;
  let payload;
  if (nodes.length > 2000 && directChildren.length > 1 && depth < 3) {
    const childDirectory = path.join(directory, basename);
    const childRelativeDirectory = `${relativeDirectory}/${basename}`;
    fs.mkdirSync(childDirectory, { recursive: true });
    payload = {
      id,
      name: root.name,
      type: root.type,
      nodeCount: nodes.length,
      root: nodes[0],
      sections: directChildren.map((child, childIndex) =>
        writeFigmaSection({
          root: child,
          index: childIndex,
          directory: childDirectory,
          relativeDirectory: childRelativeDirectory,
          children,
          compact,
          depth: depth + 1,
        }),
      ),
    };
  } else {
    payload = { id, name: root.name, type: root.type, nodes };
  }
  fs.writeFileSync(path.join(directory, filename), JSON.stringify(payload));
  return {
    id,
    name: root.name,
    type: root.type,
    nodeCount: nodes.length,
    evidence: `${relativeDirectory}/${filename}`,
    sectioned: Boolean(payload.sections),
  };
}
