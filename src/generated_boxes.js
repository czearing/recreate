/* Every generated box the capture records, paired with the engine fact that decides whether
   that box exists. A pseudo-element is not a property of its originating element: the user
   agent either generates the box or does not, and each pseudo-element's own specification
   states the condition. `::before` and `::after` exist when `content` produces something.
   `::backdrop` takes no `content` and is generated for exactly the elements in the top
   layer, so testing content there would record a phantom scrim on every element.

   Stated here once so the probe that reverts these boxes, both node-record producers and
   every consumer read one list. A further pseudo-element is one entry.

   The content arrives as a thunk, not a value, because reading it is the expensive half.
   Resolving a pseudo-element's computed style is a separate layout-sensitive read the engine
   cannot share with the element's own, and it runs once per box on every element of every
   page. An entry deciding existence from the element alone never forces that read, which is
   what stops a box almost no page has from costing one on every element that lacks it. */
const specGeneratedBoxes = {
  '::before': (element, content) => content() !== '',
  '::after': (element, content) => content() !== '',
  '::backdrop': element => element.matches(':modal')
};
/* Every other pseudo-element exists because an author wrote a rule for one, and the platform
   offers no way to ask which ones a document wrote: `getComputedStyle` and `Element.pseudo()`
   are both lookups that already need the name. So the names come from the only place holding
   them, the document's own selectors. A longer list written above would be this same defect
   with more entries in it; reading the page is what makes the answer a criterion.

   The subject selector travels with the name because it is what bounds the cost. Probing
   every element under every authored name costs a style resolution and a whole property
   enumeration per element per name, which on a page authoring five of them is the entire run
   budget. Probing an element only when it matches the selector that authored the box makes
   the work follow the authored rules rather than the node count, and a page authoring none
   pays nothing. A `matches` call resolves no style, which is why this is affordable where
   comparing against the baseline is not - and comparing against the baseline is also
   circular, because the baseline is measured only for boxes already admitted.

   A subject the recreation cannot match at rest is dropped rather than approximated. Dynamic
   states are stripped because the element is not in one now, and a rule with no subject at
   all conditions the whole document rather than any element, so it belongs to a mechanism
   this one does not own. */
const DYNAMIC_STATE = /:(?:hover|focus-visible|focus-within|focus|active|visited|target)\b/g;
const PSEUDO_ELEMENT_START = /::[\w-]/;
const authoredBoxes = (selectorText, into) => {
  for (const part of selectorText.split(',')) {
    const at = part.search(PSEUDO_ELEMENT_START);
    if (at < 0) continue;
    const suffix = `::${part.slice(at + 2).match(/^[\w-]+/)[0]}`;
    const subject = part.slice(0, at).replace(DYNAMIC_STATE, '').trim();
    if (!subject || specGeneratedBoxes[suffix]) continue;
    into.set(suffix, (into.get(suffix) || new Set()).add(subject));
  }
};
/* Style rules wherever they are written: inside a condition, inside a nesting parent and
   inside an imported sheet. A sheet the page may not read throws on `cssRules` and is skipped
   rather than aborting the scan, because one unreadable sheet must not cost the page every
   pseudo-element the readable ones authored. */
const eachAuthoredSelector = (sheet, visit, seen) => {
  if (!sheet || seen.has(sheet)) return;
  seen.add(sheet);
  let rules;
  try { rules = sheet.cssRules; } catch { return; }
  const descend = list => {
    for (const rule of Array.from(list || [])) {
      if (rule.styleSheet) eachAuthoredSelector(rule.styleSheet, visit, seen);
      else if (rule.selectorText) visit(rule.selectorText);
      if (rule.cssRules && !rule.styleSheet) descend(rule.cssRules);
    }
  };
  descend(rules);
};
/* Constructed sheets are adopted rather than listed, and a shadow root holds its own of both
   kinds, so none of them appear in `document.styleSheets`. A scope's own `styleSheets` is the
   engine's answer for that scope, which is what makes a `<link>` inside a shadow root - which
   no search for `style` elements finds - count as an authored sheet like any other. */
const documentSheets = () => {
  const sheets = [];
  for (const scope of treeScopes()) {
    sheets.push(
      ...Array.from(scope.styleSheets || []),
      ...Array.from(scope.adoptedStyleSheets || [])
    );
  }
  return sheets;
};
let authored = null;
const authoredGeneratedBoxes = () => {
  if (authored) return authored;
  authored = new Map();
  const seen = new Set();
  for (const sheet of documentSheets()) {
    eachAuthoredSelector(sheet, text => authoredBoxes(text, authored), seen);
  }
  return authored;
};
/* Each generated box this page can have, paired with the test deciding whether one element
   has it. A pseudo-element the engine generates on its own terms keeps the condition its own
   specification states; every other one exists where its author put it.

   The subjects are tested one at a time rather than joined into a selector list, for the same
   reason the revert sheet emits one rule per name: a selector list is invalid as a whole if
   any part of it is, and these parts are cut out of authored selector text, where a comma
   inside `:is()` or an attribute value splits into something that parses as neither. Joining
   would let one such fragment throw away every other element the name was authored for. */
const generatedBoxTests = () => {
  const tests = Object.entries(specGeneratedBoxes);
  for (const [suffix, subjects] of authoredGeneratedBoxes()) {
    const selectors = Array.from(subjects);
    tests.push([
      suffix,
      element => selectors.some(selector => {
        try { return element.matches(selector); } catch { return false; }
      })
    ]);
  }
  return tests;
};
/* The value that generated a box, or nothing. `none` and `normal` are the two spellings of
   "this produced no content", so a box surviving on either is one the engine generated for
   its own reasons and has no authored content to redeclare. */
const generatedContent = value =>
  value && value !== 'none' && value !== 'normal' ? value : '';
/* One element's box under `suffix`, resolved at most once however often it is asked for, so
   deciding existence and recording the box share a single read. */
const generatedBox = (element, suffix) => {
  let style;
  const styleOf = () => (style ??= getComputedStyle(element, suffix));
  return { styleOf, content: () => generatedContent(styleOf().content) };
};
/* A pseudo-element the page authored a rule for and the engine then declined to describe.
   `getComputedStyle` answers a lookup for a pseudo-element it does not implement with an empty
   declaration block rather than an error, which is what keeps an unknown or misspelled name
   free: it reduces to nothing and is dropped. The same answer comes back for a name the engine
   does implement but keeps inside its own shadow tree - the vendor-prefixed widget internals -
   and there the page really did author something the recreation will not have. The two are
   indistinguishable from here, so neither is guessed at: the name is reported and the reader
   is told the rules for it are missing rather than left to infer it from their absence. */
const declinedBoxes = new Set();
const recreatePseudoBlockers = () =>
  declinedBoxes.size
    ? [`the engine reported no style for ${Array.from(declinedBoxes).sort().join(', ')}; \
authored rules for those pseudo-elements are missing`]
    : [];
/* The generated boxes one element had, keyed by the selector suffix naming each, with every
   style reduced against that box's own baseline.

   A box that reduces to nothing is not recorded. Its rule would emit no declaration, so the
   only thing keeping it would do is make the record differ between two elements the output
   cannot tell apart — and the record is what the generated class is keyed on, so an empty box
   splits a class into two identical ones. */
const recreatePseudos = element => {
  const pseudos = {};
  for (const [suffix, exists] of generatedBoxTests()) {
    const box = generatedBox(element, suffix);
    if (!exists(element, box.content)) continue;
    const live = styleMap(box.styleOf());
    const content = box.content();
    const style = authoredStyles(live, pseudoBaselineOf(element, suffix));
    if (content || Object.keys(style).length) pseudos[suffix] = { content, style };
    else if (!Object.keys(live).length) declinedBoxes.add(suffix);
  }
  return pseudos;
};
