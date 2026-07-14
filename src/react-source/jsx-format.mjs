export const VOID_TAGS = new Set([
  'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link',
  'meta', 'param', 'source', 'track', 'wbr',
]);

const ATTRIBUTE_NAMES = {
  class: 'className',
  for: 'htmlFor',
  tabindex: 'tabIndex',
  readonly: 'readOnly',
  maxlength: 'maxLength',
  minlength: 'minLength',
  colspan: 'colSpan',
  rowspan: 'rowSpan',
  crossorigin: 'crossOrigin',
  srcset: 'srcSet',
  'fill-opacity': 'fillOpacity',
  'stroke-width': 'strokeWidth',
  'stroke-linecap': 'strokeLinecap',
  'stroke-linejoin': 'strokeLinejoin',
  'stop-color': 'stopColor',
  'stop-opacity': 'stopOpacity',
  'clip-path': 'clipPath',
  'fill-rule': 'fillRule',
  'clip-rule': 'clipRule',
};
const BOOLEAN_ATTRIBUTES = new Set([
  'allowFullScreen', 'autoFocus', 'autoPlay', 'checked', 'controls',
  'default', 'defer', 'disabled', 'formNoValidate', 'hidden', 'loop',
  'multiple', 'muted', 'noValidate', 'open', 'playsInline', 'readOnly',
  'required', 'reversed', 'selected',
]);
const NUMERIC_ATTRIBUTES = new Set([
  'colSpan', 'cols', 'maxLength', 'minLength', 'rowSpan', 'rows', 'size',
  'span', 'start', 'tabIndex',
]);

function styleObject(value) {
  const entries = String(value || '').split(';').flatMap((entry) => {
    const separator = entry.indexOf(':');
    if (separator < 0) return [];
    const property = entry.slice(0, separator).trim();
    const rawValue = entry.slice(separator + 1).trim();
    if (!property || !rawValue) return [];
    const key = property.startsWith('--')
      ? JSON.stringify(property)
      : property.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
    return [`${key}: ${JSON.stringify(rawValue)}`];
  });
  return entries.length ? `{{ ${entries.join(', ')} }}` : undefined;
}

export function reactAttributeName(name) {
  return ATTRIBUTE_NAMES[name] || name;
}

export function renderAttributes(attrs = [], dynamic = new Map()) {
  return attrs.flatMap(({ name, value }) => {
    if (name.startsWith('on') || name === 'integrity' || name === 'nonce') return [];
    const reactName = reactAttributeName(name);
    const prop = dynamic.get(name);
    if (prop) return [`${reactName}={${prop}}`];
    if (reactName === 'style') {
      const style = styleObject(value);
      return style ? [`style=${style}`] : [];
    }
    if (BOOLEAN_ATTRIBUTES.has(reactName)) return value === 'false' ? [] : [reactName];
    if (reactName === 'contentEditable' || reactName === 'spellCheck') {
      return [`${reactName}={${value !== 'false'}}`];
    }
    if (NUMERIC_ATTRIBUTES.has(reactName) && Number.isFinite(Number(value))) {
      return [`${reactName}={${Number(value)}}`];
    }
    return [`${reactName}=${JSON.stringify(value)}`];
  }).join(' ');
}

export function jsxText(value) {
  return String(value || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/{/g, '&#123;')
    .replace(/}/g, '&#125;');
}
