//! The tree-scope half of the document doubles: what a scope is, what it holds, and how a rule
//! comes to be in force inside one.
//!
//! Kept apart from any one double because more than one of them needs it, and because the
//! property under test is a property of scopes rather than of the elements a given double
//! models: a scope's own view of itself stops at the next boundary, so "the elements here" and
//! "the sheets in force here" are answerable per scope and not otherwise. An element therefore
//! knows the scope it was reached through, since whether it was measured under a declared
//! condition depends on that scope alone.

/// Expects an `Element` class in scope whose instances carry `children`, `scope` and `parent`.
pub(crate) const SCOPES: &str = r#"
const lightDescendants = node => {
  const found = [];
  for (const child of node.children) found.push(child, ...lightDescendants(child));
  return found;
};
class ShadowRoot {
  constructor(host){
    this.host = host;
    this.children = [];
    this.adoptedStyleSheets = [];
    // A scope's own sheets are the ones its tree holds; these fixtures author none inside a
    // shadow root, so the list is empty rather than absent. Absent is what no scope ever is.
    this.styleSheets = [];
    this.scope = this;
  }
  add(child){ child.parent = this.host; child.scope = this; this.children.push(child); return child; }
  querySelectorAll(selector){
    return lightDescendants(this).filter(element => element.matches(selector));
  }
}
// Constructing one sheet and adopting it everywhere is the point of the carrier, so the fixture
// counts constructions rather than adoptions: a delivery that parsed the rules once per scope
// would reach just as far and cost a page-sized multiple to do it.
globalThis.CSSStyleSheet = class CSSStyleSheet {
  constructor(){ globalThis.sheets += 1; }
  replaceSync(text){ this.text = text; }
};
// Both carriers a page can declare a rule through, so a fixture answers on where a rule is in
// force rather than on how it got there: a sheet the scope adopted, and a style element inside
// the scope. Neither reaches a scope it is not in.
globalThis.declaredIn = (scope, styleElements) => [
  ...(scope.adoptedStyleSheets || []).map(sheet => sheet.text || ''),
  ...styleElements.map(node => node.textContent || '')
];
"#;
