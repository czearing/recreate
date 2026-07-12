const number = (value) =>
  Number.isInteger(value) ? String(value) : Number(value.toFixed(4)).toString();

export function geometryBlobToSvg(blob) {
  const view = new DataView(blob.buffer, blob.byteOffset, blob.byteLength);
  const commands = [];
  let offset = 0;
  const point = () => {
    const x = view.getFloat32(offset, true);
    const y = view.getFloat32(offset + 4, true);
    offset += 8;
    return `${number(x)} ${number(y)}`;
  };
  while (offset < blob.length) {
    const command = blob[offset++];
    if (command === 0) {
      commands.push('Z');
    } else if (command === 1) {
      commands.push(`M${point()}`);
    } else if (command === 2) {
      commands.push(`L${point()}`);
    } else if (command === 3) {
      commands.push(`Q${point()} ${point()}`);
    } else if (command === 4) {
      commands.push(`C${point()} ${point()} ${point()}`);
    } else {
      return { d: null, error: `Unsupported geometry command ${command}.` };
    }
  }
  return { d: commands.join(' '), error: null };
}

export function resolveFigmaGeometry(paths, blobs) {
  if (!paths) return undefined;
  return paths.map((path) => {
    const index = path.commandsBlob;
    if (!Number.isInteger(index) || !blobs[index]?.length) {
      return {
        windingRule: path.windingRule,
        styleId: path.styleID,
        commandsBlob: index,
        d: null,
        error: 'Missing geometry blob.',
      };
    }
    return {
      windingRule: path.windingRule,
      styleId: path.styleID,
      commandsBlob: index,
      ...geometryBlobToSvg(blobs[index]),
    };
  });
}
