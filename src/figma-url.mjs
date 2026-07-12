export function parseFigmaSource(rawUrl) {
  if (!rawUrl) return null;
  let url;
  try {
    url = new URL(rawUrl);
  } catch {
    return null;
  }
  if (!/(?:^|\.)figma\.com$/i.test(url.hostname)) return null;
  const publicUrl = new URL(url);
  for (const key of publicUrl.searchParams.keys()) {
    if (!['node-id', 'page-id'].includes(key)) {
      publicUrl.searchParams.set(key, ':value');
    }
  }
  const community = url.pathname.match(/^\/community\/file\/(\d+)(?:\/([^/?#]+))?/i);
  if (community) {
    const fileId = community[1];
    return {
      kind: 'figma-community',
      fileId,
      sourceUrl: url.href,
      publicUrl: publicUrl.href,
      captureUrl:
        `https://embed.figma.com/file/${fileId}/hf_embed` +
        '?community_viewer=true&embed_host=site-spec&kind=file&page-selector=0&viewer=1',
      canvasUrl: `https://embed.figma.com/community/file/${fileId}/canvas`,
      imageBatchUrl:
        `https://embed.figma.com/community/file/${fileId}/image/batch`,
    };
  }
  const cloud = url.pathname.match(/^\/(?:design|file|proto)\/([^/?#]+)/i);
  if (cloud) {
    return {
      kind: 'figma-cloud',
      fileKey: cloud[1],
      nodeId: url.searchParams.get('node-id'),
      sourceUrl: url.href,
      publicUrl: publicUrl.href,
      captureUrl: url.href,
      canvasUrl: null,
      imageBatchUrl: null,
    };
  }
  return {
    kind: 'figma-unknown',
    sourceUrl: url.href,
    publicUrl: publicUrl.href,
  };
}
