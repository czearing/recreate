import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';

const copyArtifact = (sourceRoot, outDir, relativePath) => {
  if (!relativePath) return undefined;
  const source = path.resolve(sourceRoot, relativePath);
  if (!fs.existsSync(source) || !fs.statSync(source).isFile()) return undefined;
  const directory = path.dirname(relativePath);
  const hash = createHash('sha256').update(fs.readFileSync(source)).digest('hex').slice(0, 16);
  const filename = `merged-${hash}${path.extname(relativePath)}`;
  const targetRelative = path.join(directory, filename).replaceAll('\\', '/');
  const target = path.join(outDir, targetRelative);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(source, target);
  return targetRelative;
};

const copySnapshotAssets = (sourceRoot, outDir) => {
  const source = path.join(sourceRoot, 'snapshot-assets');
  if (!fs.existsSync(source)) return;
  const target = path.join(outDir, 'snapshot-assets');
  fs.mkdirSync(target, { recursive: true });
  for (const entry of fs.readdirSync(source, { withFileTypes: true })) {
    if (!entry.isFile()) continue;
    fs.copyFileSync(
      path.join(source, entry.name),
      path.join(target, entry.name),
    );
  }
};

export function mergeSpecStates({ states, specPaths, outDir }) {
  const merged = [...states];
  const capturedPaths = new Set(merged
    .map((state) => state.triggerElement?.path)
    .filter(Boolean));
  let nextIndex = Math.max(-1, ...merged.map((state) => state.index ?? -1)) + 1;
  for (const [specIndex, input] of specPaths.entries()) {
    const specPath = path.resolve(input);
    const sourceRoot = fs.statSync(specPath).isDirectory()
      ? specPath
      : path.dirname(specPath);
    const file = fs.statSync(specPath).isDirectory()
      ? path.join(specPath, 'spec.json')
      : specPath;
    const spec = JSON.parse(fs.readFileSync(file, 'utf8'));
    copySnapshotAssets(sourceRoot, outDir);
    for (const [stateIndex, state] of (spec.pages || []).entries()) {
      const triggerPath = state.triggerElement?.path;
      if (!triggerPath || capturedPaths.has(triggerPath)) continue;
      const evidence = copyArtifact(sourceRoot, outDir, state.evidence);
      const evidenceByViewport = Object.fromEntries(
        Object.entries(state.evidenceByViewport || {}).flatMap(([viewport, artifact]) => {
          const copied = copyArtifact(sourceRoot, outDir, artifact);
          return copied ? [[viewport, copied]] : [];
        }),
      );
      merged.push({
        ...state,
        index: nextIndex,
        evidence,
        evidenceByViewport,
        html: copyArtifact(sourceRoot, outDir, state.html),
        stylesheet: copyArtifact(sourceRoot, outDir, state.stylesheet),
        screenshot: copyArtifact(sourceRoot, outDir, state.screenshot),
        mergedFrom: file,
      });
      nextIndex += 1;
      capturedPaths.add(triggerPath);
    }
  }
  return merged;
}
