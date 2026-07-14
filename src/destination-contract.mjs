const CONTROL_HINTS = [
  { match: (node) => node.tag === 'button', primitive: 'Fluent Button' },
  { match: (node) => node.role === 'menu', primitive: 'Fluent Menu' },
  { match: (node) => node.role === 'menuitem', primitive: 'Fluent MenuItem' },
  { match: (node) => node.role === 'listbox', primitive: 'Fluent Combobox/Listbox' },
  { match: (node) => node.role === 'dialog', primitive: 'Fluent Dialog' },
  { match: (node) => node.role === 'switch', primitive: 'Fluent Switch' },
  { match: (node) => node.role === 'tab', primitive: 'Fluent Tab' },
  { match: (node) => node.tag === 'textarea', primitive: 'Native textarea' },
  { match: (node) => node.tag === 'svg', primitive: 'Bebop icon' },
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
    requiredPackages: ['@1js/bebop-icons', '@1js/fluentui-modern'],
    forbiddenDelivery: [
      'iframe embedding',
      'redirect to generated reconstruction',
      'site-spec-runtime.js in shipping source',
      '__site-spec reconstruction routes in shipping source',
      'site-spec-manifest.json or ORACLE_ONLY.txt in shipping source',
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
      '--require @1js/bebop-icons,@1js/fluentui-modern ' +
      '--matrix <site-spec-output/acceptance-matrix.json> ' +
      '--report <structured-acceptance-report.json> ' +
      '--comparisons <native-comparisons.json>',
  };
}
