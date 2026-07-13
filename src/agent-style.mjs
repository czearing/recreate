const STYLE_KEYS = [
  'display', 'position', 'inset', 'top', 'right', 'bottom', 'left',
  'width', 'height', 'minWidth', 'maxWidth', 'minHeight', 'maxHeight',
  'margin', 'padding', 'gap', 'rowGap', 'columnGap', 'flex', 'flexGrow',
  'flexShrink', 'flexBasis', 'flexDirection', 'flexWrap', 'justifyContent',
  'alignItems', 'alignSelf', 'gridTemplateColumns', 'gridTemplateRows',
  'gridAutoFlow', 'overflow', 'overflowX', 'overflowY', 'boxSizing',
  'color', 'background', 'backgroundColor', 'backgroundImage',
  'backgroundSize', 'backgroundPosition', 'backgroundRepeat', 'border',
  'borderRadius', 'boxShadow', 'opacity', 'transform', 'fontFamily',
  'fontSize', 'fontWeight', 'fontStyle', 'lineHeight', 'letterSpacing',
  'textAlign', 'textTransform', 'whiteSpace', 'objectFit', 'objectPosition',
];

const kebab = (value) =>
  value.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);

export function pickAgentStyle(source) {
  return Object.fromEntries(STYLE_KEYS.flatMap((key) => {
    const value = source?.[key] ?? source?.[kebab(key)];
    return value == null || value === '' ? [] : [[key, value]];
  }));
}

export function styleDelta(style, parentStyle, forceFull = false) {
  const current = pickAgentStyle(style);
  if (forceFull) return current;
  const parent = pickAgentStyle(parentStyle);
  return Object.fromEntries(
    Object.entries(current).filter(([key, value]) => parent[key] !== value),
  );
}
