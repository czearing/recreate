const number = (value) =>
  Number.isInteger(value) ? String(value) : Number(value.toFixed(4)).toString();
const point = (value) => `${number(value.x)} ${number(value.y)}`;

function decode(blob) {
  const view = new DataView(blob.buffer, blob.byteOffset, blob.byteLength);
  let offset = 0;
  const readUint = () => {
    const value = view.getUint32(offset, true);
    offset += 4;
    return value;
  };
  const readFloat = () => {
    const value = view.getFloat32(offset, true);
    offset += 4;
    return value;
  };
  const vertexCount = readUint();
  const segmentCount = readUint();
  const regionCount = readUint();
  const vertices = [];
  for (let index = 0; index < vertexCount; index += 1) {
    readUint();
    vertices.push({ x: readFloat(), y: readFloat() });
  }
  const segments = [];
  for (let index = 0; index < segmentCount; index += 1) {
    readUint();
    segments.push({
      start: readUint(),
      tangentStart: { x: readFloat(), y: readFloat() },
      end: readUint(),
      tangentEnd: { x: readFloat(), y: readFloat() },
    });
  }
  const regions = [];
  for (let index = 0; index < regionCount; index += 1) {
    const windingRule = readUint() === 0 ? 'EVENODD' : 'NONZERO';
    const loops = [];
    const loopCount = readUint();
    for (let loopIndex = 0; loopIndex < loopCount; loopIndex += 1) {
      const segmentIndexes = [];
      const count = readUint();
      for (let item = 0; item < count; item += 1) {
        segmentIndexes.push(readUint());
      }
      loops.push(segmentIndexes);
    }
    regions.push({ windingRule, loops });
  }
  return { vertices, segments, regions };
}

function appendSegment(commands, segment, vertices, forward) {
  const start = forward ? vertices[segment.start] : vertices[segment.end];
  const end = forward ? vertices[segment.end] : vertices[segment.start];
  const startTangent = forward
    ? segment.tangentStart
    : segment.tangentEnd;
  const endTangent = forward
    ? segment.tangentEnd
    : segment.tangentStart;
  if (
    startTangent.x === 0 &&
    startTangent.y === 0 &&
    endTangent.x === 0 &&
    endTangent.y === 0
  ) {
    commands.push(`L${point(end)}`);
  } else {
    commands.push(
      `C${point({
        x: start.x + startTangent.x,
        y: start.y + startTangent.y,
      })} ${point({
        x: end.x + endTangent.x,
        y: end.y + endTangent.y,
      })} ${point(end)}`,
    );
  }
}

function regionPath(region, network) {
  const commands = [];
  for (const loop of region.loops) {
    if (!loop.length) continue;
    const first = network.segments[loop[0]];
    let current = first.start;
    if (loop.length > 1) {
      const second = network.segments[loop[1]];
      if (first.end !== second.start && first.end !== second.end) {
        current = first.end;
      }
    }
    commands.push(`M${point(network.vertices[current])}`);
    for (const segmentIndex of loop) {
      const segment = network.segments[segmentIndex];
      const forward = segment.start === current;
      appendSegment(commands, segment, network.vertices, forward);
      current = forward ? segment.end : segment.start;
    }
    commands.push('Z');
  }
  return { windingRule: region.windingRule, d: commands.join(' ') };
}

function openPaths(network) {
  const adjacency = new Map();
  network.segments.forEach((segment, index) => {
    for (const vertex of [segment.start, segment.end]) {
      const entries = adjacency.get(vertex) || [];
      entries.push(index);
      adjacency.set(vertex, entries);
    }
  });
  const starts = [...adjacency]
    .filter(([, entries]) => entries.length === 1)
    .map(([vertex]) => vertex);
  if (!starts.length && network.segments.length) {
    starts.push(network.segments[0].start);
  }
  const visited = new Set();
  const paths = [];
  for (const start of starts) {
    let current = start;
    const commands = [`M${point(network.vertices[current])}`];
    while (true) {
      const index = (adjacency.get(current) || [])
        .find((item) => !visited.has(item));
      if (index == null) break;
      visited.add(index);
      const segment = network.segments[index];
      const forward = segment.start === current;
      appendSegment(commands, segment, network.vertices, forward);
      current = forward ? segment.end : segment.start;
    }
    if (commands.length > 1) {
      paths.push({ windingRule: 'NONZERO', d: commands.join(' ') });
    }
  }
  return paths;
}

export function resolveVectorNetwork(node, blobs) {
  const data = node.vectorData;
  const index = data?.vectorNetworkBlob;
  if (!Number.isInteger(index)) return undefined;
  if (!blobs[index]?.length) {
    return { blobIndex: index, paths: [], error: 'Missing vector network blob.' };
  }
  try {
    const network = decode(blobs[index]);
    const normalized = data.normalizedSize;
    if (normalized?.x && normalized?.y && node.size?.x && node.size?.y) {
      const scaleX = node.size.x / normalized.x;
      const scaleY = node.size.y / normalized.y;
      for (const vertex of network.vertices) {
        vertex.x *= scaleX;
        vertex.y *= scaleY;
      }
      for (const segment of network.segments) {
        segment.tangentStart.x *= scaleX;
        segment.tangentStart.y *= scaleY;
        segment.tangentEnd.x *= scaleX;
        segment.tangentEnd.y *= scaleY;
      }
    }
    return {
      blobIndex: index,
      normalizedSize: normalized,
      vertexCount: network.vertices.length,
      segmentCount: network.segments.length,
      regionCount: network.regions.length,
      paths: network.regions.length
        ? network.regions.map((region) => regionPath(region, network))
        : openPaths(network),
      error: null,
    };
  } catch (error) {
    return { blobIndex: index, paths: [], error: String(error) };
  }
}
