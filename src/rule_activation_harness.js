// A CSSOM double for the capture walk, so the walk's real logic runs under Node instead of
// a browser. The double decides which at-rule conditions match, which is exactly what a
// browser decides, so a walk that assumes conditions match cannot pass against it.
const CSSRule = { MEDIA_RULE: 4 };
// The platform's own line between a rule that groups style rules and one that merely has
// children: @media, @supports, @container, @layer and @scope are CSSGroupingRule, while
// @keyframes is not. The walk descends by that test, so the double has to model it.
class CSSGroupingRule {}
const authoredSheetTexts = [];
const pathOf = element => element.path;

const scene = __SCENE__;

const elements = scene.elements.map(spec => ({ ...spec, probes: {}, shadowRoot: null }));

const matchesSelector = (selector, element) =>
  selector
    .split(',')
    .map(part => part.trim())
    .some(part => part === '*' || (part.startsWith('.') && element.classes.includes(part.slice(1))));

const makeStyle = declarations => {
  const names = Object.keys(declarations);
  return {
    getPropertyValue: name => declarations[name] || '',
    getPropertyPriority: () => '',
    [Symbol.iterator]: function* () {
      yield* names;
    }
  };
};

const declarationText = declarations =>
  Object.entries(declarations)
    .map(([name, value]) => `${name}: ${value};`)
    .join(' ');

const buildRule = spec => {
  // A layer order statement (`@layer a, b;`) is a rule with a prelude and no block, so it
  // exposes neither `selectorText` nor `cssRules`. Modelling it keeps the double honest
  // about the one rule shape that carries ordering and nothing else.
  if (spec.statement) {
    return { type: 12, cssText: spec.statement };
  }
  if (spec.selectorText) {
    return {
      selectorText: spec.selectorText,
      cssText: `${spec.selectorText} { ${declarationText(spec.declarations)} }`,
      style: makeStyle(spec.declarations)
    };
  }
  // A definition rule such as @property or @counter-style has a block of descriptors and
  // no children at all, so it exposes neither `selectorText` nor `cssRules`.
  if (!spec.rules) {
    return { type: 12, cssText: `${spec.prelude} { ${declarationText(spec.declarations)} }` };
  }
  const rules = spec.rules.map(buildRule);
    const grouped = {
    type: spec.media ? CSSRule.MEDIA_RULE : 12,
    conditionText: spec.conditionText,
    cssText: `${spec.prelude} { ${rules.map(rule => rule.cssText).join(' ')} }`,
    cssRules: rules
  };  // A keyframes block exposes children without grouping style rules, which is the shape
  // that must not be descended into.
  return spec.keyframes ? grouped : Object.assign(new CSSGroupingRule(), grouped);
};

// Applies a probe block the way a browser would: the sentinel lands only on elements the
// selector matches and for which every enclosing condition holds.
const applyProbeBlock = text => {
  const conditions = [];
  let rest = text.trim();
  while (rest.startsWith('@')) {
    const open = rest.indexOf('{');
    conditions.push(rest.slice(0, open).trim());
    rest = rest.slice(open + 1, rest.lastIndexOf('}')).trim();
  }
  const open = rest.indexOf('{');
  const selector = rest.slice(0, open).trim();
  const property = rest.slice(open + 1, rest.lastIndexOf('}')).split(':')[0].trim();
  for (const element of elements) {
    if (!matchesSelector(selector, element)) continue;
    const holds = conditions.every(condition => (scene.matching[condition] || []).includes(element.path));
    if (holds) element.probes[property] = '1';
  }
};

const document = {
  styleSheets: scene.sheets.map(sheet => ({ cssRules: sheet.map(buildRule) })),
  adoptedStyleSheets: [],
  head: {
    appendChild: node => {
      for (const block of node.textContent.split('\n')) applyProbeBlock(block);
    }
  },
  createElement: () => ({ textContent: '', remove: () => {} }),
  querySelectorAll: selector => elements.filter(element => matchesSelector(selector, element))
};

const getComputedStyle = element => ({
  getPropertyValue: name => element.probes[name] || ''
});

__CAPTURE__

console.log(JSON.stringify({ cssRules, stateStyles }));
