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
const pseudoElements = {
  '::before': (element, content) => content() !== '',
  '::after': (element, content) => content() !== '',
  '::backdrop': element => element.matches(':modal')
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
/* The generated boxes one element had, keyed by the selector suffix naming each, with every
   style reduced against that box's own baseline. */
const recreatePseudos = element => {
  const pseudos = {};
  for (const [suffix, exists] of Object.entries(pseudoElements)) {
    const box = generatedBox(element, suffix);
    if (!exists(element, box.content)) continue;
    pseudos[suffix] = {
      content: box.content(),
      style: authoredStyles(styleMap(box.styleOf()), pseudoBaselineOf(element, suffix))
    };
  }
  return pseudos;
};
