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
// A style rule is not a grouping rule — measured in Edge, where
// `CSSStyleRule.prototype instanceof CSSGroupingRule` is false — yet CSS Nesting gives it
// children all the same. Modelling it as its own class is what lets the walk ask "does this
// rule contain rules" and "is this rule a record" as the two separate questions they are.
class CSSStyleRule {}
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
    // A style rule serialises its nested children inside its own block, which is why a child
    // recorded again would publish the same declarations twice.
    const nested = (spec.rules || []).map(child => buildRule(child, conditions));
    const body = [declarationText(spec.declarations || {}), ...nested.map(child => child.cssText)]
      .filter(Boolean)
      .join(' ');
    const rule = Object.assign(new CSSStyleRule(), {
      selectorText: spec.selectorText,
      cssText: `${spec.selectorText} { ${body} }`,
      cssRules: nested,
      style
    });
    for (const child of nested) child.parentRule = rule;
    liveRules.push({ selectorText: spec.selectorText, style, conditions, rule });
    return rule;
  }
  // A run of declarations sitting after a nested rule is wrapped in a nested declarations
  // rule, which carries a block but no selector of its own and matches exactly what its
  // parent matches. It is the one rule shape whose subject is nowhere in its own text. It is
  // deliberately absent from `liveRules`, which is keyed by selector text it does not have.
  if (spec.nestedDeclarations) {
    return { cssText: declarationText(spec.nestedDeclarations), style: makeStyle(spec.nestedDeclarations) };
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
  const built = spec.keyframes ? grouped : Object.assign(new CSSGroupingRule(), grouped);
  // Every rule knows what encloses it, which is the only route to the subject of a rule whose
  // own text does not state one.
  for (const child of rules) child.parentRule = built;
  return built;
};
