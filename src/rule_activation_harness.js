// A CSSOM double for the capture walk, so the walk's real logic runs under Node instead of
// a browser. This half is the page: which elements exist, which at-rule conditions hold for
// each, and what `getComputedStyle` therefore answers. The rule and sheet objects it reads
// come from `rule_activation_cssom.js`, which is concatenated ahead of this file.
//
// The double decides which at-rule conditions match, which is exactly what a browser
// decides, so a walk that assumes conditions match cannot pass against it.
const pathOf = element => element.path;

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

// Which elements an at-rule prelude holds for, answered by the scene exactly as a browser
// answers it from the viewport or from a container's used size.
const conditionHolds = (prelude, element) =>
  (scene.matching[prelude] || []).includes(element.path);

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
    if (conditions.every(condition => conditionHolds(condition, element))) {
      element.probes[property] = '1';
    }
  }
};

const document = {
  // A sheet with no location inherits this, which is what CSSOM's null `href` means.
  baseURI: 'http://harness.test/index.html',
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
  // A probe sentinel is read straight back; anything else is resolved through the rules
  // still in force, so a block a stage empties stops contributing exactly as it would.
  getPropertyValue: name => {
    if (element.probes[name]) return element.probes[name];
    let value = '';
    for (const { selectorText, style, conditions, rule } of liveRules) {
      if (rule.parentStyleSheet && rule.parentStyleSheet.disabled) continue;
      if (!matchesSelector(selectorText, element)) continue;
      if (!conditions.every(prelude => conditionHolds(prelude, element))) continue;
      const declared = style.getPropertyValue(name);
      if (declared) value = declared;
    }
    return value;
  }
});

// The records the walk would have built, paired with the elements they came from.
const elementNodes = elements.map(element => [element, { path: element.path }]);

__CAPTURE__

console.log(JSON.stringify({
  cssRules,
  stateStyles,
  parses,
  reads,
  shorthands: [...shorthandBlocks.values()],
  decided: elementNodes.map(([, node]) => node),
  // Each sheet's switch once the walk is over, so a stage that turns a sheet off to rewrite
  // its rules can be shown to have left it as the page had it.
  switches: document.styleSheets.map(sheet => sheet.disabled),
  // What every style rule's block holds once the walk is over, so a stage that withdraws a
  // block to read past it can be shown to have put the page back as it found it.
  blocks: liveRules.map(rule => ({ selectorText: rule.selectorText, cssText: rule.style.cssText }))
}));
