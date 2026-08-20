// The sheet and document half of the capture walk's double. A sheet is more than the rules
// it lists, so it is modelled apart from them; the rule objects it hands out are built by the
// half concatenated before this one.

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
