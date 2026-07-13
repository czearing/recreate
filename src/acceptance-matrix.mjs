const viewportKey = (viewport) =>
  `${viewport.width}x${viewport.height}@${viewport.dpr || 1}`;

export function buildAcceptanceMatrix({ states, viewports, components }) {
  const stateCells = states.flatMap((state) => {
    const evidence = state.evidenceByViewport || {};
    return viewports.map((viewport) => {
      const key = `${viewport.width}x${viewport.height}`;
      return {
        id: `state-${state.index ?? 'home'}-${viewportKey(viewport)}`,
        stateIndex: state.index,
        type: state.type,
        viewport,
        url: state.url,
        evidence: evidence[key] || state.evidence,
        required: [
          'visible hierarchy',
          'text and accessibility identity',
          'geometry within one pixel',
          'computed style constraints',
          'asset identity',
        ],
      };
    });
  });
  const interactionCells = states
    .filter((state) => state.index >= 0)
    .flatMap((state) => viewports.map((viewport) => ({
      id: `interaction-${state.index}-${viewportKey(viewport)}`,
      stateIndex: state.index,
      viewport,
      trigger: state.trigger,
      action: state.probe?.action,
      destination: state.url,
      required: ['activation', 'resulting state', 'focus', 'dismissal or restoration'],
    })));
  return {
    schemaVersion: 1,
    purpose: 'Required native-delivery cells. Every cell must pass before PR or delivery.',
    stateCells,
    interactionCells,
    componentCells: components.filter((component) => component.file).map((component) => ({
      id: component.id,
      label: component.identity.label,
      evidence: component.file,
      required: ['native primitive mapping', 'desktop geometry', 'mobile geometry'],
    })),
    deliveryCells: [
      'no iframe',
      'no captured HTML injection',
      'no generated reconstruction runtime',
      'no redirect or link to reconstruction output',
      'required destination-native packages imported',
    ],
  };
}
