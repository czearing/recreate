const clean = (value, limit = 160) =>
  String(value || '').replace(/\s+/g, ' ').trim().slice(0, limit);

const within = (pathValue, rootPath) =>
  pathValue === rootPath || pathValue?.startsWith(`${rootPath}>`);

const unique = (items, key) => [
  ...new Map(items.map((item) => [key(item), item])).values(),
];

function coverage(items, componentRoots, describe) {
  const required = unique(items, (item) => item.path || describe(item));
  const missing = required
    .filter((item) => !componentRoots.some((root) => within(item.path, root)))
    .map(describe);
  return {
    required: required.length,
    covered: required.length - missing.length,
    missing,
  };
}

export function buildGenerationReadiness({
  capture,
  components,
  states,
  viewports,
  crawlRequested,
}) {
  const componentRoots = components
    .filter((component) => component.file)
    .map((component) => component.path);
  const visibleText = (capture?.nodes || []).filter((node) =>
    node.visible && node.tag === '#text' && clean(node.text),
  );
  const controls = (capture?.behaviors || []).filter((behavior) =>
    clean(behavior.label || behavior.href || behavior.path),
  );
  const assets = (capture?.exactAssets || []).filter((asset) => asset.path);
  const animations = [
    ...(capture?.animationElements || []),
    ...(capture?.lifecycleAnimation?.tracks || []),
  ].filter((animation) => animation.path);
  const regions = components.filter((component) => !component.file);
  const text = coverage(visibleText, componentRoots, (node) => ({
    path: node.path,
    text: clean(node.text),
  }));
  const controlCoverage = coverage(controls, componentRoots, (control) => ({
    path: control.path,
    label: clean(control.label || control.href),
    tag: control.tag,
    role: control.role,
  }));
  const assetCoverage = coverage(assets, componentRoots, (asset) => ({
    path: asset.path,
    type: asset.type,
    src: clean(asset.currentSrc || asset.src || asset.file),
  }));
  const animationCoverage = coverage(animations, componentRoots, (animation) => ({
    path: animation.path,
    type: clean(animation.type || animation.tag || 'animation'),
  }));
  const stateCount = states.length;
  const interactionCount = states.filter((state) => state.index >= 0).length;
  const failures = [
    regions.length && `${regions.length} component region(s) lack readable shards`,
    text.missing.length && `${text.missing.length} visible text node(s) lack component ownership`,
    controlCoverage.missing.length && `${controlCoverage.missing.length} control(s) lack component ownership`,
    assetCoverage.missing.length && `${assetCoverage.missing.length} asset(s) lack component ownership`,
    animationCoverage.missing.length && `${animationCoverage.missing.length} animation track(s) lack component ownership`,
    crawlRequested && interactionCount === 0 && 'crawl requested but no interaction states were captured',
    viewports.length < 2 && 'fewer than two responsive viewports were captured',
  ].filter(Boolean);
  return {
    schemaVersion: 1,
    purpose: 'Whole-site generation readiness. Do not claim readiness while any failure remains.',
    ready: failures.length === 0,
    failures,
    totals: {
      componentCandidates: components.length,
      readableComponents: componentRoots.length,
      states: stateCount,
      interactions: interactionCount,
      viewports: viewports.length,
    },
    coverage: {
      regions: {
        required: components.length,
        covered: componentRoots.length,
        missing: regions.map((component) => ({
          id: component.id,
          path: component.path,
          label: component.identity.label,
          nodeCounts: component.nodeCounts,
        })),
      },
      text,
      controls: controlCoverage,
      assets: assetCoverage,
      animations: animationCoverage,
    },
  };
}
