import fs from 'node:fs';
import path from 'node:path';
import { decodeFigmaKiwi } from './figma-kiwi.mjs';
import { writeFigmaEvidence } from './figma-evidence.mjs';
import { localizeFigmaImages } from './figma-assets.mjs';
import { loadCdpResource } from './figma-resource.mjs';

export async function captureFigmaSpec({
  cdp,
  frameId,
  outDir,
  source,
  requestedUrl,
  profile,
}) {
  const raw = await loadCdpResource(cdp, frameId, source.canvasUrl);
  const decoded = decodeFigmaKiwi(Uint8Array.from(raw.bytes));
  const figma = writeFigmaEvidence({
    outDir,
    source,
    decoded,
    byteLength: raw.bytes.length,
    profile,
  });
  figma.assets = await localizeFigmaImages({
    cdp,
    frameId,
    outDir,
    source,
    nodes: decoded.message.nodeChanges,
  });
  fs.writeFileSync(
    path.join(outDir, 'figma.json'),
    JSON.stringify(figma, null, 2),
  );
  if (profile === 'full') {
    fs.writeFileSync(
      path.join(outDir, 'evidence', 'figma', 'canvas.fig'),
      raw.bytes,
    );
  }
  const implementation = {
    schemaVersion: 2,
    purpose: 'Agent-facing Figma implementation blueprint.',
    source: {
      requestedUrl,
      capturedUrl: source.captureUrl,
      sourceType: source.kind,
      capturedAt: new Date().toISOString(),
    },
    profile,
    readOrder: [
      'implementation.json',
      'figma.json',
      'the active evidence/figma/<page>.json',
      'evidence/figma/variables.json',
      'the matching evidence/figma/values/<prefix>.json for each {$ref}',
    ],
    rules: [
      'Implement native destination components; do not recreate Figma editor chrome.',
      'Use decoded design nodes, hierarchy, geometry, styles, variables, assets, and prototype interactions.',
      'Load only the active page evidence; full files may contain tens of thousands of nodes.',
      'Resolve {$ref:<hash>} values through the hash-prefix shard in figma.json.',
    ],
    figma,
    validation: {
      passed:
        figma.nodeCount > 0 &&
        figma.pages.length > 0 &&
        figma.assets.complete,
      errors: figma.assets.errors,
    },
  };
  fs.writeFileSync(
    path.join(outDir, 'implementation.json'),
    JSON.stringify(implementation, null, 2),
  );
  fs.writeFileSync(
    path.join(outDir, 'spec.json'),
    JSON.stringify({
      schemaVersion: 2,
      source: implementation.source,
      profile,
      figma,
      validation: implementation.validation,
    }, null, 2),
  );
  fs.writeFileSync(
    path.join(outDir, 'summary.json'),
    JSON.stringify({
      source: implementation.source,
      profile,
      figma,
      validation: implementation.validation,
    }, null, 2),
  );
  return implementation;
}
