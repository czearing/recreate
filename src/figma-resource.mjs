export async function loadCdpResource(cdp, frameId, url) {
  const loaded = await cdp.send('Network.loadNetworkResource', {
    frameId,
    url,
    options: { disableCache: true, includeCredentials: true },
  });
  if (!loaded.resource.success || !loaded.resource.stream) {
    throw new Error(
      `Resource request failed: ${loaded.resource.httpStatusCode || 'unknown status'}`,
    );
  }
  const buffers = [];
  try {
    while (true) {
      const part = await cdp.send('IO.read', {
        handle: loaded.resource.stream,
        size: 1024 * 1024,
      });
      buffers.push(Buffer.from(part.data, part.base64Encoded ? 'base64' : 'utf8'));
      if (part.eof) break;
    }
  } finally {
    await cdp.send('IO.close', { handle: loaded.resource.stream }).catch(() => {});
  }
  return {
    bytes: Buffer.concat(buffers),
    headers: loaded.resource.headers || {},
  };
}
