import fs from 'node:fs';
import path from 'node:path';
import { loadCdpResource } from './figma-resource.mjs';

const hashToHex = (hash) => {
  if (!hash) return null;
  if (typeof hash === 'string') {
    return /^[a-f0-9]{40}$/i.test(hash) ? hash.toLowerCase() : null;
  }
  const bytes = hash instanceof Uint8Array ? hash : Object.values(hash);
  if (
    bytes.length !== 20 ||
    bytes.some((value) => !Number.isInteger(Number(value)) || value < 0 || value > 255)
  ) return null;
  return Array.from(
    bytes,
    (value) => Number(value).toString(16).padStart(2, '0'),
  ).join('');
};

export function collectFigmaImageHashes(nodes) {
  const hashes = new Set();
  const seen = new WeakSet();
  const visit = (value) => {
    if (!value || typeof value !== 'object' || ArrayBuffer.isView(value)) return;
    if (seen.has(value)) return;
    seen.add(value);
    const hash = hashToHex(value.image?.hash);
    if (hash) hashes.add(hash);
    for (const child of Object.values(value)) {
      visit(child);
    }
  };
  for (const node of nodes) visit(node);
  return [...hashes].sort();
}

async function resolveImageUrls(cdp, source, hashes) {
  if (!hashes.length) return {};
  const expression = `(async () => {
    const response = await fetch(${JSON.stringify(source.imageBatchUrl)}, {
      method: 'POST',
      credentials: 'include',
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json',
        'X-Csrf-Bypass': 'yes'
      },
      body: JSON.stringify({
        sha1s: ${JSON.stringify(hashes)},
        needs_compressed_textures: false
      })
    });
    if (!response.ok) throw new Error('Figma image batch failed: ' + response.status);
    const body = await response.json();
    return body.meta?.s3_urls || {};
  })()`;
  const result = await cdp.send('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (result.exceptionDetails) {
    throw new Error(result.exceptionDetails.text || 'Figma image batch failed.');
  }
  return result.result.value;
}

const extensionFor = (headers, bytes) => {
  const type = String(headers['content-type'] || headers['Content-Type'] || '');
  if (type.includes('png')) return 'png';
  if (type.includes('webp')) return 'webp';
  if (type.includes('svg')) return 'svg';
  if (type.includes('gif')) return 'gif';
  if (bytes[0] === 0x89 && bytes[1] === 0x50) return 'png';
  if (bytes[0] === 0xff && bytes[1] === 0xd8) return 'jpg';
  if (bytes.toString('ascii', 0, 4) === 'RIFF') return 'webp';
  if (bytes.toString('ascii', 0, 3) === 'GIF') return 'gif';
  return 'jpg';
};

export async function localizeFigmaImages({
  cdp,
  frameId,
  outDir,
  source,
  nodes,
}) {
  const hashes = collectFigmaImageHashes(nodes);
  const errors = [];
  let urls = {};
  try {
    urls = await resolveImageUrls(cdp, source, hashes);
  } catch (error) {
    errors.push({ stage: 'resolve', error: String(error) });
  }
  const assetDir = path.join(outDir, 'snapshot-assets');
  fs.mkdirSync(assetDir, { recursive: true });
  const assets = {};
  for (let start = 0; start < hashes.length; start += 8) {
    const batch = hashes.slice(start, start + 8);
    const localized = await Promise.all(batch.map(async (hash) => {
      const url = urls[hash];
      if (!url) {
        return { hash, error: 'Figma image batch omitted this hash.' };
      }
      try {
        const resource = await loadCdpResource(cdp, frameId, url);
        const filename = `${hash}.${extensionFor(resource.headers, resource.bytes)}`;
        fs.writeFileSync(path.join(assetDir, filename), resource.bytes);
        return {
          hash,
          file: `snapshot-assets/${filename}`,
          byteLength: resource.bytes.length,
        };
      } catch (error) {
        return { hash, error: String(error) };
      }
    }));
    for (const result of localized) {
      if (result.error) {
        errors.push(result);
      } else {
        assets[result.hash] = {
          file: result.file,
          byteLength: result.byteLength,
        };
      }
    }
  }
  fs.writeFileSync(
    path.join(outDir, 'evidence', 'figma', 'assets.json'),
    JSON.stringify({ assets, errors }, null, 2),
  );
  return {
    count: Object.keys(assets).length,
    byteLength: Object.values(assets)
      .reduce((total, asset) => total + asset.byteLength, 0),
    manifest: 'evidence/figma/assets.json',
    complete: errors.length === 0,
    errors,
  };
}
