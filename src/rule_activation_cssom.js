// The CSSOM half of the capture walk's double: the rule and sheet objects a browser exposes,
// built from a scene's JSON. It models the three things the walk's decisions turn on — that a
// declaration block stores longhands rather than the shorthand the author wrote, that rule
// objects are stable across reads of a sheet, and that a block's text round-trips through
// `cssText` — and nothing else.
//
// The scene is declared here because both halves read it, and this half is concatenated first.
const scene = __SCENE__;

const CSSRule = { MEDIA_RULE: 4 };
// The platform's own line between a rule that groups style rules and one that merely has
// children: @media, @supports, @container, @layer and @scope are CSSGroupingRule, while
// @keyframes is not. The walk descends by that test, so the double has to model it.
class CSSGroupingRule {}
// An @import is the one rule whose child is a whole sheet rather than a rule list, and it
// is deliberately not a CSSGroupingRule — which is why the walk saw it as a leaf.
class CSSImportRule {}

// Every style rule the double has built, with the conditions enclosing it. A declaration
// reaches an element when the rule's selector matches and every enclosing condition holds,
// which is the whole of the cascade a stage that withdraws a block and reads again can see.
const liveRules = [];

const declarationText = declarations =>
  Object.entries(declarations)
    .map(([name, value]) => `${name}: ${value};`)
    .join(' ');

const parseDeclarations = text =>
  Object.fromEntries(
    text
      .split(';')
      .map(part => part.trim())
      .filter(Boolean)
      .map(part => [
        part.slice(0, part.indexOf(':')).trim(),
        part.slice(part.indexOf(':') + 1).trim()
      ])
  );

// A declaration block stores longhands: a shorthand is a parsing convenience the engine does
// not retain, so iterating a block yields the longhands it set while asking for the shorthand
// by name re-serialises them. A scene spells that division as `expanded`, which is the only
// part of CSS grammar a double cannot derive and a browser always can.
//
// `cssText` round-trips through the same text a browser serialises, so emptying a block and
// assigning its saved text back is the same operation here as there.
const makeStyle = (declarations, expanded) => {
  let own = { ...declarations };
  let stored = { ...(expanded || declarations) };
  const division = { ...(expanded || declarations) };
  return {
    get cssText() {
      return declarationText(own);
    },
    set cssText(text) {
      own = parseDeclarations(text);
      stored = Object.keys(own).length ? { ...division } : {};
    },
    getPropertyValue: name => (name in stored ? stored[name] : own[name]) || '',
    getPropertyPriority: () => '',
    get length() {
      return Object.keys(stored).length;
    },
    item(slot) {
      return Object.keys(stored)[slot] || '';
    },
    [Symbol.iterator]: function* () {
      yield* Object.keys(stored);
    }
  };
};

// CSSOM rule objects are stable: reading a sheet twice yields the same rules, which is what
// makes a rule the walk holds the same rule a later stage withdraws.
const builtRules = new Map();
const buildRule = (spec, conditions = []) => {
  if (!builtRules.has(spec)) builtRules.set(spec, makeRule(spec, conditions));
  return builtRules.get(spec);
};

const makeRule = (spec, conditions) => {
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
    const style = makeStyle(spec.declarations, spec.expanded);
    const rule = {
      selectorText: spec.selectorText,
      cssText: `${spec.selectorText} { ${declarationText(spec.declarations)} }`,
      style
    };
    liveRules.push({ selectorText: spec.selectorText, style, conditions, rule });
    return rule;
  }
  // A definition rule such as @property, @font-face or @position-try has a block of
  // descriptors and no children at all, so it exposes neither `selectorText` nor `cssRules`.
  // Chrome reports a `length` on such a block but no indexed names, so iterating it yields
  // `undefined` — the shape that must never be enumerated.
  if (!spec.rules) {
    const cssText = `${spec.prelude} { ${declarationText(spec.declarations)} }`;
    if (!spec.descriptors) return { type: 12, cssText };
    const style = makeStyle(spec.declarations);
    style.item = () => undefined;
    return { type: 12, cssText, style };
  }
  const rules = spec.rules.map(rule => buildRule(rule, conditions.concat(spec.prelude)));
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
// Every rule a sheet holds names that sheet, however deeply an at-rule encloses it. A stage
// that switches a sheet off to rewrite its rules reaches them all through this link.
const stampSheet = (rule, sheet) => {
  if (rule.selectorText) rule.parentStyleSheet = sheet;
  for (const nested of rule.cssRules || []) stampSheet(nested, sheet);
};

let reads = 0;
const buildSheet = spec => {
  const plain = Array.isArray(spec);
  const sheet = {
    href: plain ? null : spec.href || null,
    media: { mediaText: plain ? '' : spec.media || '' },
    // A sheet the page itself switched off must be found switched off afterwards, so the
    // double carries the switch rather than assuming every sheet starts on.
    disabled: plain ? false : !!spec.disabled,
    get cssRules() {
      reads++;
      if (!plain && spec.unreadable) throw new Error('SecurityError');
      return (plain ? spec : spec.rules).map(rule => {
        const built = buildRule(rule);
        stampSheet(built, sheet);
        return built;
      });
    }
  };
  return sheet;
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
    this.cssRules = ((scene.parsed || {})[text] || []).map(rule => buildRule(rule));
  }
}
