/* Which declarations are worth recording is decided by measurement, not by a list of
   property names. `all: revert` rolls an element back to the user-agent origin in place,
   so the value it then reports is what the element would compute if the author had
   declared nothing: user-agent rules for its real tag and inheritance from its real
   parent are both already in the answer. A live value equal to that baseline carries no
   information, because the recreation keeps the tag and recomputes the same value for
   free. `initial` and `unset` cannot be used here - `initial` gives the spec value, so
   `display` on a div would read `inline`, and `unset` discards the user-agent origin
   entirely. Only `revert` lands on the origin the recreation also runs under.
   The style attribute is author origin and outranks author stylesheet rules of equal
   importance, so an important inline declaration beats even an authored `!important`.
   `all` does not cover `direction`, `unicode-bidi` or custom properties; those compare
   equal to themselves and are pruned. Custom properties are owned elsewhere. */
/* The engine enumerates a logical alias beside every physical longhand it resolves to -
   `padding-inline-start` next to `padding-left`, `inline-size` next to `width`,
   `border-end-end-radius` next to `border-bottom-right-radius` - so recording both
   writes one declaration twice. Logical names are built from flow-relative segments and
   physical names are built from side segments, so the physical spelling is selected by
   the naming grammar rather than by a list of pairs. Physical values are already
   resolved for the page's writing mode, so nothing is lost by preferring them. */
const FLOW_RELATIVE = ['inline', 'block', 'start', 'end'];
/* The locale property is not authored in CSS at all: the engine derives it from the
   `lang` attribute, which the recreation reproduces verbatim on the same element. Its
   baseline therefore differs for a reason no stylesheet can express, and recording it
   would write the attribute a second time in a syntax nothing reads. */
const DERIVED_FROM_ATTRIBUTE = '-webkit-locale';
const redundantProperty = property => {
  if (property === DERIVED_FROM_ATTRIBUTE) return true;
  const segments = property.split('-');
  return FLOW_RELATIVE.some(segment => segments.includes(segment));
};
const styleMap = style => {
  const values = {};
  for (const property of style) {
    if (!redundantProperty(property)) values[property] = style.getPropertyValue(property);
  }
  return values;
};
/* Many properties fall back to the used colour when nothing declares them - borders,
   outlines, carets, text decoration and the rest - so recording each one beside `color`
   writes the same colour ten times and the recreation re-derives every one of them from
   `color` alone. Which properties those are is not listed but measured: a property
   tracks the colour when it equals `color` both with the author's CSS and without it. A
   property whose fallback is anything else - a background is transparent, a tap
   highlight has its own value - fails the baseline half and is kept. A colour authored
   to match `color` is dropped and re-derived identically. */
const tracksColor = (property, live, baseline) =>
  property !== 'color' &&
  live[property] === live.color &&
  baseline[property] === baseline.color;
const authoredStyles = (live, baseline) => {
  const values = {};
  for (const property in live) {
    if (live[property] !== baseline[property] && !tracksColor(property, live, baseline)) {
      values[property] = live[property];
    }
  }
  return values;
};
const elementBaselines = new WeakMap();
const pseudoBaselines = new WeakMap();
const baselineOf = element => elementBaselines.get(element) || {};
const pseudoBaselineOf = (element, name) => (pseudoBaselines.get(element) || {})[name] || {};
/* Inheritance is one-way, so reverting every element at one depth leaves the parents it
   inherits from untouched. Writing a whole level and then reading it costs one style
   recalculation per level rather than one per element, which is what keeps this inside
   the run budget on a deep page. A pseudo-element has its own cascade, so reverting the
   originating element does not reach it; one stylesheet reverts every pseudo at once,
   which is sound in a single pass because a pseudo inherits from an element that still
   holds its real values. */
/* Reverting an element removes the declarations that gave it a scroll range - its size
   and its `overflow` - so the engine clamps any offset it was holding to zero, and
   putting the style attribute back does not restore it. The offset is the one fact only
   this capture can observe, since it lives in neither markup nor computed style, so the
   probe records it alongside the style attribute and puts both back. Offsets are read
   once before any write, and written once after every restoration, so the interleaving
   costs no extra layout. */
const measureBaselines = (root, skip) => {
  const levels = [];
  const collect = (element, depth) => {
    if (skip(element)) return;
    (levels[depth] = levels[depth] || []).push(element);
    for (const child of element.children) collect(child, depth + 1);
    if (element.shadowRoot) {
      for (const child of element.shadowRoot.children) collect(child, depth + 1);
    }
  };
  collect(root, 0);
  const scrolled = [];
  for (const level of levels) {
    for (const element of level) {
      const left = element.scrollLeft;
      const top = element.scrollTop;
      if (left || top) scrolled.push([element, left, top]);
    }
  }
  const sheet = document.createElement('style');
  sheet.textContent = '*::before,*::after{all:revert !important}';
  document.head.appendChild(sheet);
  for (const level of levels) {
    for (const element of level) {
      pseudoBaselines.set(element, {
        '::before': styleMap(getComputedStyle(element, '::before')),
        '::after': styleMap(getComputedStyle(element, '::after'))
      });
    }
  }
  sheet.remove();
  for (const level of levels) {
    const saved = level.map(element => element.getAttribute('style'));
    for (const element of level) element.style.setProperty('all', 'revert', 'important');
    for (const element of level) elementBaselines.set(element, styleMap(getComputedStyle(element)));
    level.forEach((element, index) => {
      if (saved[index] === null) element.removeAttribute('style');
      else element.setAttribute('style', saved[index]);
    });
  }
  for (const [element, left, top] of scrolled) element.scrollTo(left, top);
};
