//! The page double the scoped-rules tests measure against.
//!
//! It is a file of its own because it is the fixture, not the assertions. It models the four
//! things delivery depends on and nothing else: nested tree scopes, the sheets each scope
//! already holds, the ordered list of cascade layers those sheets declare, and the two ways a
//! sheet refuses to be written to — a cross-origin sheet that will not hand back its rules at
//! all, and the grammar rule that nothing may be inserted ahead of an `@import`.

use crate::node_eval;

/// `log` separates "installed everywhere" from "installed everywhere eventually": a read that
/// runs before the last scope adopted is a read taken under a partial condition.
///
/// `layerOrder` reads the layer names out of the sheets a scope holds, in the order the sheets
/// hold them and adopted sheets last, which is the order the cascade sorts them in. It is
/// derived from the page's own text rather than restated, so a test can say "before every layer
/// the page declares" without naming one.
pub(crate) const DOUBLE: &str = r#"
globalThis.log = [];
class CSSStyleSheet {
  replaceSync(text){ this.text = text; }
}
globalThis.CSSStyleSheet = CSSStyleSheet;
const sheetOf = (texts, unreadable) => {
  const made = {
    written: texts.map(cssText => ({ cssText })),
    get cssRules(){
      if (unreadable) throw new Error('cross-origin sheet');
      return made.written;
    },
    get text(){ return made.written.map(rule => rule.cssText).join('\n'); },
    insertRule(text, slot){
      if (unreadable) throw new Error('cross-origin sheet');
      // Nothing may precede an @import, and CSSOM reports that by throwing.
      if (made.written.slice(slot).some(rule => rule.cssText.startsWith('@import'))) {
        throw new Error('HierarchyRequestError');
      }
      made.written.splice(slot, 0, { cssText: text });
      return slot;
    },
    deleteRule(slot){ made.written.splice(slot, 1); }
  };
  return made;
};
globalThis.sheet = (...texts) => sheetOf(texts, false);
globalThis.unreadableSheet = () => sheetOf([], true);
const scope = name => {
  const made = {
    name,
    hosts: [],
    styleSheets: [],
    _adopted: [],
    get adoptedStyleSheets(){ return this._adopted; },
    set adoptedStyleSheets(next){
      globalThis.log.push(name + ':' + next.map(sheet => sheet.text).join('|'));
      this._adopted = next;
    },
    querySelectorAll: () => made.hosts
  };
  return made;
};
const document = scope('document');
const outer = scope('outer');
const inner = scope('inner');
const sibling = scope('sibling');
document.hosts = [{ shadowRoot: outer }, { shadowRoot: sibling }, { shadowRoot: null }];
outer.hosts = [{ shadowRoot: inner }];
globalThis.everyScope = [document, outer, inner, sibling];
globalThis.inForce = () =>
  globalThis.everyScope.map(each => each.adoptedStyleSheets.map(sheet => sheet.text));
globalThis.rulesHeld = () =>
  globalThis.everyScope.map(each => each.styleSheets.map(sheet => sheet.text));
globalThis.layerOrder = each => {
  const order = [];
  for (const sheet of [...each.styleSheets, ...each.adoptedStyleSheets]) {
    for (const found of sheet.text.matchAll(/@layer\s+([^\s;{]+)/g)) {
      if (!order.includes(found[1])) order.push(found[1]);
    }
  }
  return order;
};
"#;

/// Runs `body` against the double and the shipped delivery source, and reports what
/// `expression` evaluated to.
pub(crate) fn evaluate(body: &str, expression: &str) -> serde_json::Value {
    node_eval::evaluate(
        &format!("{DOUBLE}\n{}\n{body}", crate::scoped_rules::SOURCE),
        expression,
    )
}
