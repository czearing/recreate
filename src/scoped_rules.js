/* Every tree scope on the page, outermost first, as this reading found it.

   A shadow root is a scope of its own: it holds its own stylesheets, and none of the document's
   reach into it. Three readers here need that same list - which sheets the page authored, which
   scopes a declared condition has to hold in, and which scopes hold rules to re-emit - and each
   had derived it separately, so a scope one of them entered was not necessarily one the others
   did. Asking once is what stops those answers drifting apart.

   `querySelectorAll` is a scope's own view of itself and stops at the next boundary, so the
   recursion is what crosses one, and a root nested inside a root is reached by the same step
   that reaches the first.

   Enumerated once per evaluation, because one evaluation of this script is one reading of one
   page state and the scopes are part of that state, exactly as the authored box list is. The
   walk is the whole document, so answering it once is the difference between one traversal per
   reading and one per reader - and on a page large enough for that to matter, the readers are
   the same three every time. */
let scopes = null;
const treeScopes = () => {
  if (scopes) return scopes;
  scopes = [];
  const enter = scope => {
    scopes.push(scope);
    for (const element of scope.querySelectorAll('*')) {
      if (element.shadowRoot) enter(element.shadowRoot);
    }
  };
  enter(document);
  return scopes;
};
/* Reading the page under a declared condition - every element rolled back to the user-agent
   origin, every transition declared away - for exactly the duration of that read.

   A rule reaches the tree scope whose stylesheet holds it and no other, so a condition declared
   in one scope answers for that scope alone, and everything read in the scopes it missed answers
   as though the condition had never been declared at all. Nothing reports that: the read
   succeeds, and hands back the page's own live values, which is exactly what a page with nothing
   to say looks like. A style attribute carries a condition onto a node whatever scope the node
   is in, because it is a property of the node rather than a selector match, but a pseudo-element
   has no node and no style attribute, so a rule is the only carrier it has - and declaring that
   rule in every scope is the only way its measurement reaches as far as an element's.

   Reaching a scope is not being in force in it. A rule the page outranks was delivered and did
   nothing, and reports that to nobody, for the same reason a rule delivered to the wrong scope
   does. A selector cannot be made unbeatable: whatever weight it is given, one more compound
   beats it, so an important author rule on `.card::before` defeats a universal one and the
   measurement reads the page's live value back as its own baseline. The style attribute the
   element half uses does not win by being heavier - it wins because a weight is never asked
   for. The same place exists for a rule: the cascade sorts by origin and importance, then by
   cascade layer, and only then by specificity, and for important declarations the layer order
   reverses, so an important declaration in the first layer outranks every other author
   declaration whatever its selector. `revert` still rolls back the whole origin rather than the
   layer, so what the rules measure is unchanged.

   Being first is a position, not a name: layers sort in the order they are first declared. The
   rules travel inside the layer, so a scope that refuses the claim still declares the layer
   where the constructed sheet sits - behind the page's own layers, ahead of everything it left
   unlayered - rather than falling back to a second mechanism.

   The carrier is a constructed sheet, adopted per scope. Adopting inserts no node into a tree
   the page owns, so it raises no mutation record and is invisible both to the page's own
   observers and to this capture's mutation-based settle gate, which appending a `<style>` to
   each root would not be. The assignment replaces the list rather than mutating it in place,
   which is the spelling that holds whether the property is a mutable array or the frozen one it
   used to be. Splicing the sheet back out and deleting the claimed rule leave every scope
   holding what it was found holding, and both are found by identity rather than by position,
   because the page's own code runs during the read and may insert either. */
const MEASUREMENT_LAYER = 'recreate-measurement';
/* Nothing may be inserted ahead of an `@import`, and a sheet the page loaded from another origin
   will not be read at all; CSSOM reports both by throwing. Neither is a reason to stop, so the
   claim takes the earliest position that is offered to it - a later slot in the first sheet is
   still earlier than every later sheet. */
const claimLayer = scope => {
  for (const sheet of scope.styleSheets) {
    let rules;
    try {
      rules = sheet.cssRules;
    } catch (unreadable) {
      continue;
    }
    for (let slot = 0; slot <= rules.length; slot += 1) {
      try {
        sheet.insertRule(`@layer ${MEASUREMENT_LAYER};`, slot);
        return [sheet, rules[slot]];
      } catch (refused) {
        continue;
      }
    }
  }
  return null;
};
const releaseLayer = claim => {
  if (!claim) return;
  const [sheet, rule] = claim;
  const slot = Array.prototype.indexOf.call(sheet.cssRules, rule);
  if (slot >= 0) sheet.deleteRule(slot);
};
const underRules = (text, read) => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`@layer ${MEASUREMENT_LAYER}{${text}}`);
  const scopes = treeScopes();
  const claims = scopes.map(claimLayer);
  for (const scope of scopes) scope.adoptedStyleSheets = [...scope.adoptedStyleSheets, sheet];
  try {
    return read();
  } finally {
    for (const scope of scopes) {
      scope.adoptedStyleSheets = scope.adoptedStyleSheets.filter(each => each !== sheet);
    }
    for (const claim of claims) releaseLayer(claim);
  }
};
