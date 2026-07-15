const CONTROL_HINTS = [
  { match: (node) => node.tag === 'button', primitive: 'Native design-system button' },
  { match: (node) => node.role === 'menu', primitive: 'Native design-system menu' },
  { match: (node) => node.role === 'menuitem', primitive: 'Native design-system menu item' },
  { match: (node) => node.role === 'listbox', primitive: 'Native design-system listbox' },
  { match: (node) => node.role === 'dialog', primitive: 'Native design-system dialog' },
  { match: (node) => node.role === 'switch', primitive: 'Native design-system switch' },
  { match: (node) => node.role === 'tab', primitive: 'Native design-system tab' },
  { match: (node) => node.tag === 'textarea', primitive: 'Native textarea' },
  { match: (node) => node.tag === 'svg', primitive: 'Native design-system icon' },
];

export function buildPrimitiveInventory(nodes) {
  const counts = {};
  for (const node of nodes) {
    for (const hint of CONTROL_HINTS) {
      if (!hint.match(node)) continue;
      counts[hint.primitive] = (counts[hint.primitive] || 0) + 1;
      break;
    }
  }
  return Object.entries(counts)
    .map(([primitive, count]) => ({ primitive, count }))
    .sort((left, right) => right.count - left.count);
}

export function destinationContract() {
  return {
    mode: 'native-required',
    requiredPackages: [],
    forbiddenDelivery: [
      'iframe embedding',
      'redirect to generated reconstruction',
      'recreate-runtime.js in shipping source',
      '__recreate reconstruction routes in shipping source',
      'recreate-manifest.json or ORACLE_ONLY.txt in shipping source',
      'captured application scripts',
    ],
    acceptance: [
      'Every visible control maps to a destination-native primitive or documented native exception.',
      'Structured desktop/mobile geometry stays within one pixel.',
      'Every captured interaction and restoration path passes.',
      'A structured acceptance report proves every required state and viewport before delivery.',
    ],
    validator:
      'node src/validate-native-implementation.mjs --root <implementation-root> ' +
      '--paths <destination-source-roots> ' +
      '--matrix <recreate-output/acceptance-matrix.json> ' +
      '--report <structured-acceptance-report.json> ' +
      '--comparisons <native-comparisons.json>',
  };
}
