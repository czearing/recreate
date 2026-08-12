// A browser-free stand-in for the part of CSS the capture scripts read: the value in force
// at an element, and whether the element is drawn at all.
//
// Elements are `{ parent, declarations }`, so a double can express a declaration sitting on
// an ancestor rather than on the element under test — which is the only shape the defect
// this models appears in. The three ways to leave an element undrawn each resolve by a
// different rule, and reading any one of them the way another resolves is wrong in one
// direction: `visibility` inherits, so a descendant can re-show a hidden subtree; `opacity`
// composites the subtree away and no descendant can undo it; `display: none` removes the
// box outright. That is exactly why the shipped rule delegates to `checkVisibility` instead
// of reading properties. Every verdict here was confirmed against the capture browser.

const INITIAL = {
  position: 'static',
  'z-index': 'auto',
  'pointer-events': 'auto',
  display: 'block',
  visibility: 'visible',
  opacity: '1'
};
const INHERITED = new Set(['visibility']);

const inForce = (element, property) => {
  const own = element.declarations[property];
  if (own !== undefined) return own;
  if (INHERITED.has(property) && element.parent) return inForce(element.parent, property);
  return INITIAL[property];
};

const drawn = (element, options) => {
  if (options.visibilityProperty && inForce(element, 'visibility') === 'hidden') return false;
  for (let node = element; node; node = node.parent) {
    if (node.declarations.display === 'none') return false;
    if (options.opacityProperty && node.declarations.opacity === '0') return false;
  }
  return true;
};

// Gives one element the two surfaces the shipped rule reads it through.
globalThis.recreateStyled = element => {
  element.parent = element.parent || null;
  element.checkVisibility = (options = {}) => drawn(element, options);
  return element;
};

globalThis.getComputedStyle = element => new Proxy({}, {
  get: (_, name) => inForce(element, String(name).replace(/[A-Z]/g, c => '-' + c.toLowerCase()))
});
