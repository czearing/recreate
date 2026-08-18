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

   The carrier is a constructed sheet, adopted per scope. Adopting inserts no node into a tree
   the page owns, so it raises no mutation record and is invisible both to the page's own
   observers and to this capture's mutation-based settle gate, which appending a `<style>` to
   each root would not be. A scope's adopted sheets are ordered after the sheets it holds
   itself, so these rules win every tie they enter, and splicing the sheet back out leaves every
   scope holding what it was found holding. The assignment replaces the list rather than
   mutating it in place, which is the spelling that holds whether the property is a mutable
   array or the frozen one it used to be. */
const underRules = (text, read) => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(text);
  const scopes = treeScopes();
  for (const scope of scopes) scope.adoptedStyleSheets = [...scope.adoptedStyleSheets, sheet];
  try {
    return read();
  } finally {
    for (const scope of scopes) {
      scope.adoptedStyleSheets = scope.adoptedStyleSheets.filter(each => each !== sheet);
    }
  }
};
