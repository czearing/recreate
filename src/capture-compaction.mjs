const rounded = (value) =>
  typeof value === 'number' ? Math.round(value * 100) / 100 : value;
export const compactRect = (rect = {}) =>
  Object.fromEntries(
    ['x', 'y', 'width', 'height', 'right', 'bottom']
      .filter((key) => rect[key] != null)
      .map((key) => [key, rounded(rect[key])]),
  );
const styleProperties = [
  'display', 'position', 'width', 'height', 'margin', 'padding', 'gap',
  'flex-direction', 'justify-content', 'align-items', 'grid-template-columns',
  'overflow', 'z-index', 'color', 'background-color', 'background-image',
  'border', 'border-radius', 'box-shadow', 'opacity', 'transform',
  'font-family', 'font-size', 'font-weight', 'line-height', 'letter-spacing',
  'text-align',
];
const compactStyle = (style = {}) =>
  Object.fromEntries(
    styleProperties
      .filter((property) => style[property] != null && style[property] !== '')
      .map((property) => [property, style[property]]),
  );
export const compactStyleDelta = (style = {}, parentStyle = {}) => {
  const defaults = {
    position: 'static',
    margin: '0px',
    padding: '0px',
    gap: 'normal',
    'flex-direction': 'row',
    'justify-content': 'normal',
    'align-items': 'normal',
    'grid-template-columns': 'none',
    overflow: 'visible',
    'z-index': 'auto',
    'background-color': 'rgba(0, 0, 0, 0)',
    'background-image': 'none',
    border: '0px none',
    'border-radius': '0px',
    'box-shadow': 'none',
    opacity: '1',
    transform: 'none',
    'letter-spacing': 'normal',
    'text-align': 'start',
  };
  return Object.fromEntries(
    Object.entries(compactStyle(style)).filter(([property, value]) => {
      if (value === parentStyle[property]) return false;
      const fallback = defaults[property];
      if (fallback == null) return true;
      return property === 'border'
        ? !String(value).startsWith(fallback)
        : value !== fallback;
    }),
  );
};
const compactAttributes = (attrs = {}) => {
  const kept = {};
  const exactNames = new Set([
    'id', 'class', 'role', 'title', 'href', 'src', 'srcset', 'alt', 'type',
    'name', 'value', 'placeholder', 'tabindex', 'disabled', 'checked',
    'selected', 'data-testid',
  ]);
  for (const [name, rawValue] of Object.entries(attrs)) {
    if (!exactNames.has(name) && !name.startsWith('aria-')) continue;
    const value = String(rawValue);
    kept[name] = /^(?:data|blob):/i.test(value)
      ? '[asset stored in captured state HTML]'
      : value.slice(0, 500);
  }
  return kept;
};
const compactAnimation = (animation) => ({
  target: animation.target || animation.targetPath,
  id: animation.id || undefined,
  playState: animation.playState,
  currentTime: animation.currentTime,
  playbackRate: animation.playbackRate,
  timeline: animation.timeline,
  timing: animation.timing,
});
const compactAsset = ({ value: _value, dataUrl: _dataUrl, ...asset }) =>
  Object.fromEntries(Object.entries(asset).map(([key, rawValue]) => [
    key,
    typeof rawValue === 'string' && /^(?:data|blob):/i.test(rawValue)
      ? '[asset stored in captured state HTML]'
      : rawValue,
  ]));
const compactTrack = (track) => {
  const samples = track.samples || [];
  const indexes = [0, 0.25, 0.5, 0.75, 1].map((position) =>
    Math.floor((samples.length - 1) * position));
  return {
    path: track.path,
    tag: track.tag,
    data: track.data,
    samples: [...new Set(indexes)].filter((index) => samples[index]).map((index) => ({
      time: rounded(samples[index].time),
      rect: compactRect(samples[index].rect),
      style: samples[index].style,
    })),
  };
};
export const compactCapture = (capture) => {
  const nodes = implementationNodes(capture.nodes);
  const paths = new Set(nodes.map((node) => node.path));
  return {
  document: {
    url: capture.document.url,
    title: capture.document.title,
    lang: capture.document.lang,
    viewport: capture.document.viewport,
    scroll: capture.document.scroll,
    bodyStyle: compactStyle(capture.document.bodyStyle),
  },
  initialDocument: capture.initialDocument,
  nodes: nodes.map((node) => ({
    path: node.path,
    parentPath: node.parentPath,
    nodeType: node.nodeType,
    tag: node.tag,
    attrs: compactAttributes(node.attrs),
    role: node.role,
    ariaLabel: node.ariaLabel,
    text: node.text
      ? String(node.text).replace(/\s+/g, ' ').trim().slice(0, 500)
      : undefined,
    visible: node.visible || undefined,
    rect: node.visible ? compactRect(node.rect) : undefined,
    clickable: node.clickable || undefined,
    pseudoType: node.pseudoType,
    shadowRootType: node.shadowRootType,
  })),
  behaviors: capture.behaviors.filter((behavior) => paths.has(behavior.path)).map((behavior) => ({
    path: behavior.path,
    tag: behavior.tag,
    role: behavior.role,
    href: behavior.href,
    type: behavior.type,
    formAction: behavior.formAction,
    disabled: behavior.disabled || undefined,
    ariaExpanded: behavior.ariaExpanded,
    ariaPressed: behavior.ariaPressed,
    ariaSelected: behavior.ariaSelected,
    ariaHaspopup: behavior.ariaHaspopup,
    label: behavior.label,
    listeners: compactListeners(behavior.listeners),
  })),
  globalListeners: compactListeners(capture.globalListeners),
  fonts: capture.fonts,
  animations: capture.animations.map(compactAnimation),
  animationElements: (capture.animationElements || []).map((element) => ({
    path: element.path,
    tag: element.tag,
    text: element.text,
    rect: compactRect(element.rect),
    data: element.data,
    style: compactStyle(element.style),
  })),
  smoothScroll: capture.smoothScroll,
  horizontalTracks: capture.horizontalTracks,
  exactAssets: capture.exactAssets
    .filter((asset) => paths.has(asset.path))
    .map(compactAsset),
  scrollStates: capture.scrollStates.map((state) => ({
    path: state.path,
    clientHeight: state.clientHeight,
    initialScrollHeight: state.initialScrollHeight,
    maxScroll: state.maxScroll,
    checkpoints: state.checkpoints.map((checkpoint) => ({
      progress: rounded(checkpoint.progress),
      scrollTop: rounded(checkpoint.scrollTop),
      animations: (checkpoint.animations || []).map(compactAnimation),
    })),
  })),
  lifecycleAnimation: {
    durationMs: capture.lifecycleAnimation?.durationMs || 0,
    frameCount: capture.lifecycleAnimation?.frameCount || 0,
    tracks: (capture.lifecycleAnimation?.tracks || [])
      .filter((track) => track.samples?.length > 1)
      .map(compactTrack),
    animationDefinitionCount:
      capture.lifecycleAnimation?.animationDefinitions?.length || 0,
  },
  readiness: capture.readiness,
  timings: capture.timings,
  cssomBlockedStylesheetCount: capture.cssomBlockedStylesheetCount,
  };
};
import {
  compactListeners,
  implementationNodes,
} from './evidence-filter.mjs';
