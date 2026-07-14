const viewportKey = (viewport) =>
  `${viewport.width}x${viewport.height}@${viewport.dpr || 1}`;

const interactive = (control) =>
  !control.disabled && (
    ['button', 'input', 'textarea', 'select', 'a'].includes(
      String(control.tag || '').toLowerCase(),
    ) ||
    ['button', 'link', 'menuitem', 'option', 'switch', 'tab'].includes(
      String(control.role || '').toLowerCase(),
    ) ||
    control.href
  );

export function buildAcceptanceMatrix({
  states,
  viewports,
  components,
  controls = [],
  animations = [],
}) {
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
  const requiredControls = [
    ...new Map(controls.filter(interactive).map((control) => [control.path, control])).values(),
  ];
  const interactionCells = (requiredControls.length
    ? requiredControls.map((control, index) => {
      const state = states.find((candidate) =>
        candidate.index >= 0 && candidate.triggerElement?.path === control.path);
      return { control, index, state };
    })
    : states.filter((state) => state.index >= 0).map((state, index) => ({
      control: state.triggerElement || { label: state.trigger },
      index,
      state,
    }))
  ).flatMap(({ control, index, state }) => viewports.map((viewport) => ({
    id: `interaction-${String(index).padStart(3, '0')}-${viewportKey(viewport)}`,
    stateIndex: state?.index ?? null,
    viewport,
    controlPath: control.path,
    trigger: control.label || state?.trigger,
    tag: control.tag,
    role: control.role,
    action: state?.probe?.action,
    destination: state?.url,
    captured: Boolean(state),
    evidence: state?.evidenceByViewport?.[`${viewport.width}x${viewport.height}`] ||
      state?.evidence,
    required: ['activation', 'resulting state', 'focus', 'dismissal or restoration'],
  })));
  const animationTargets = [
    ...new Map(animations
      .filter((animation) => animation.path || animation.target)
      .map((animation) => [animation.path || animation.target, animation]))
      .values(),
  ];
  const animationCells = animationTargets.flatMap((animation, index) =>
    viewports.map((viewport) => ({
      id: `animation-${String(index).padStart(3, '0')}-${viewportKey(viewport)}`,
      viewport,
      path: animation.path || animation.target,
      type: animation.type || animation.tag || 'animation',
      required: ['target', 'property', 'duration', 'easing', 'keyframes or trajectory'],
    })));
  return {
    schemaVersion: 1,
    purpose: 'Required native-delivery cells. Every cell must pass before PR or delivery.',
    stateCells,
    interactionCells,
    animationCells,
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
