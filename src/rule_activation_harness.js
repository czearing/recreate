// A CSSOM double for the capture walk, so the walk's real logic runs under Node instead of
// a browser. The double decides which at-rule conditions match, which is exactly what a
// browser decides, so a walk that assumes conditions match cannot pass against it.
const CSSRule = { MEDIA_RULE: 4 };
// The platform's own line between a rule that groups style rules and one that merely has
// children: @media, @supports, @container, @layer and @scope are CSSGroupingRule, while
// @keyframes is not. The walk descends by that test, so the double has to model it.
class CSSGroupingRule {}
// An @import is the one rule whose child is a whole sheet rather than a rule list, and it
// is deliberately not a CSSGroupingRule — which is why the walk saw it as a leaf.
class CSSImportRule {}
const pathOf = element => element.path;

const scene = __SCENE__;
const authoredSheetTexts = scene.authoredSheets || [];

// A scene may seed computed values an element already reports before any probe runs, which
// is how a custom property inherited from `:root` reaches `getComputedStyle` in a browser.
const elements = scene.elements.map(spec => ({
  ...spec,
  probes: { ...spec.computed },
  shadowRoot: null
}));

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
  // `import` names the sheet the rule points at: a sheet spec, or a key into `scene.named`
  // when two rules must reach the same sheet object, which is the only way to spell a cycle.
  // `null` is the shape CSSOM requires when a supports() condition blocked the fetch.
  if ('import' in spec) {
    return Object.assign(new CSSImportRule(), {
      cssText: `@import url("${spec.imports || 'imported.css'}");`,
      styleSheet:
        spec.import == null
          ? null
          : typeof spec.import === 'string'
            ? namedSheet(spec.import)
            : buildSheet(spec.import)
    });
  }
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
    // A browser reads the rule interface from the at-rule the author wrote, so the double
    // derives it from the prelude too. Taking it from a fixture flag instead lets a scene
    // declare a `@media` prelude that is not a media rule, which no page can produce.
    type: spec.prelude.startsWith('@media') ? CSSRule.MEDIA_RULE : 12,
    conditionText: spec.conditionText,
    cssText: `${spec.prelude} { ${rules.map(rule => rule.cssText).join(' ')} }`,
    cssRules: rules
  };
  // A keyframes block exposes children without grouping style rules, which is the shape
  // that must not be descended into.
  return spec.keyframes ? grouped : Object.assign(new CSSGroupingRule(), grouped);
};

// A sheet is more than its rule list: its `media` conditions everything inside without
// appearing inside, and its `href` is the only identity a recovered text carries. A scene
// may spell a sheet as a bare rule list when it exercises neither.
//
// Reads are counted because a walk with no bound on the import graph does not crash — the
// recursion unwinds into the same catch that guards an unreadable sheet — so its only
// observable is how much work it did before silently giving up.
let reads = 0;
const buildSheet = spec => {
  const plain = Array.isArray(spec);
  return {
    href: plain ? null : spec.href || null,
    media: { mediaText: plain ? '' : spec.media || '' },
    get cssRules() {
      reads++;
      if (!plain && spec.unreadable) throw new Error('SecurityError');
      return (plain ? spec : spec.rules).map(buildRule);
    }
  };
};

// Two imports of one address are two independent sheets, so a scene names a sheet only when
// it needs both rules to reach the *same* object — which is what makes a cycle expressible.
const namedSheets = new Map();
const namedSheet = name => {
  if (!namedSheets.has(name)) namedSheets.set(name, buildSheet(scene.named[name]));
  return namedSheets.get(name);
};

// The browser parses a recovered sheet's text; the double looks up what that text parses
// to, so the walk's own decisions about the fallback are observable without a CSS parser.
// The count is reported because "did not parse this again" is the whole point of the
// fallback's guard and leaves no trace in what was recorded.
let parses = 0;
class CSSStyleSheet {
  constructor() {
    this.cssRules = [];
  }
  replaceSync(text) {
    parses++;
    this.cssRules = ((scene.parsed || {})[text] || []).map(buildRule);
  }
}

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
  styleSheets: scene.sheets.map(buildSheet),
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

console.log(JSON.stringify({ cssRules, stateStyles, parses, reads }));
