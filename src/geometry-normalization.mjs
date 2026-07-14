const round = (value) => Math.round(value * 1e6) / 1e6;

export function normalizeSnapshotGeometry(decoded, documentMetrics) {
  const expectedWidth =
    documentMetrics?.scroll?.width ||
    documentMetrics?.viewport?.width;
  if (!expectedWidth) return 1;
  const maximumWidth = Math.max(
    0,
    ...decoded.nodes
      .filter((node) => node.rect?.x === 0 && node.rect?.width > 0)
      .map((node) => node.rect.width),
  );
  const scale = maximumWidth / expectedWidth;
  if (!Number.isFinite(scale) || scale < 1.05 || scale > 4) return 1;
  for (const node of decoded.nodes) {
    if (!node.rect) continue;
    const rect = node.rect;
    node.rect = {
      x: round(rect.x / scale),
      y: round(rect.y / scale),
      width: round(rect.width / scale),
      height: round(rect.height / scale),
      right: round(rect.right / scale),
      bottom: round(rect.bottom / scale),
    };
  }
  for (const document of decoded.documents) {
    document.contentWidth = round(document.contentWidth / scale);
    document.contentHeight = round(document.contentHeight / scale);
  }
  return scale;
}
