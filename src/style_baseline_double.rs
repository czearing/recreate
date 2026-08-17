//! The document double the baseline pruning tests measure against.
//!
//! It is a file of its own because it is the fixture, not the assertions: it models the
//! parts of a browser the probe actually depends on - a declaration block that replaces
//! rather than merges, a computed style that reports whether it was read live or reverted,
//! and generated content authored per element and per pseudo name.

use crate::node_eval;
/// The double reports a value built from the inputs a reverted element actually depends on and
/// records every enumeration it serves, separated by whether it was live, reverted or a pseudo.
/// `content` is authored per element and per pseudo name so a test can move exactly one of them.
///
/// Its elements shadow `style` with a symbol, which is what a custom element declaring the class
/// field `style = v` does to the accessor it inherits: the field is installed with
/// [[DefineOwnProperty]], so it ignores the prototype chain. A probe that reaches through the
/// instance throws here for the same reason it throws on a page built out of components, and it
/// throws without a browser.
pub(crate) const DOUBLE: &str = r#"
globalThis.content = new Map();
globalThis.order = [];
globalThis.mark = stage => {
  if (globalThis.order[globalThis.order.length - 1] !== stage) globalThis.order.push(stage);
};
class Element {
  constructor(tagName){
    this.tagName = tagName;
    this.children = [];
    this.shadowRoot = null;
    this.attributes = new Map();
    this.style = Symbol('clobbered');
    this.reverted = false;
    this.modal = false;
    this.parent = null;
  }
  matches(selector){
    if (selector === ':modal') return this.modal;
    return selector.split(',').some(part => {
      const trimmed = part.trim();
      if (trimmed === '*') return true;
      if (trimmed.startsWith('#')) return this.attributes.get('id') === trimmed.slice(1);
      return trimmed.toUpperCase() === this.tagName;
    });
  }
  add(child){ child.parent = this; this.children.push(child); return child; }
  getAttribute(name){ return this.attributes.has(name) ? this.attributes.get(name) : null; }
  setAttribute(name, value){
    this.attributes.set(name, value);
    if (name !== 'style') return;
    const declared = new Map();
    for (const declaration of value.split(';')) {
      const [property, ...rest] = declaration.split(':');
      if (rest.length) declared.set(property.trim(), rest.join(':').trim());
    }
    this.reverted = declared.get('all') === 'revert !important';
    globalThis.mark(this.reverted ? 'revert' : 'restore');
  }
  removeAttribute(name){
    this.attributes.delete(name);
    if (name === 'style') { this.reverted = false; globalThis.mark('restore'); }
  }
  get scrollLeft(){ return 0; }
  get scrollTop(){ return 0; }
  scrollTo(){}
  get name(){ return this.tagName + (this.attributes.get('id') ? '#' + this.attributes.get('id') : ''); }
}
const documentElement = new Element('HTML');
const head = documentElement.add(new Element('HEAD'));
const body = documentElement.add(new Element('BODY'));
const plain = body.add(new Element('P'));
plain.setAttribute('style', 'all:unset');
const marked = body.add(new Element('P'));
marked.setAttribute('id', 'marked');
globalThis.marked = marked;
globalThis.plain = plain;
globalThis.body = body;
head.appendChild = child => { head.add(child); globalThis.sheets += 1; };
// The selectors a document authored, as the only thing the scan reads from a sheet. Tests set
// `globalThis.authoredSelectors` to a list of selector texts, which is what `cssRules` yields once the
// walk has descended through whatever conditions they were written inside.
globalThis.authoredSelectors = [];
globalThis.document = {
  documentElement,
  head,
  get styleSheets(){
    return [{ get cssRules(){ return globalThis.authoredSelectors.map(selectorText => ({ selectorText })); } }];
  },
  adoptedStyleSheets: [],
  createElement: tag => {
    const made = new Element(tag.toUpperCase());
    made.remove = () => { head.children = head.children.filter(item => item !== made); };
    return made;
  }
};
// CSSOM answers a lookup for a pseudo-element the engine does not support with an empty
// declaration block rather than an error, and a vendor-prefixed widget internal the engine
// keeps inside its own shadow tree answers the same way. Modelled because the difference
// between "described and identical to its baseline" and "not described at all" is the whole of
// what a reader can be told about a rule that did not survive.
globalThis.unsupported = new Set();
globalThis.getComputedStyle = (element, pseudo) => {
  const generated = globalThis.content.get(element.name + (pseudo || '')) || 'none';
  const described = !pseudo || !globalThis.unsupported.has(pseudo);
  let value;
  if (pseudo) {
    if (element.reverted) throw new Error('pseudo read while the element was reverted');
    globalThis.pseudoReads += 1;
    globalThis.mark('pseudo');
    value = 'pseudo:' + element.name + pseudo;
  } else if (element.reverted) {
    globalThis.measured.push(element.name);
    value = 'revert:' + element.name;
  } else {
    globalThis.live.push(element.name);
    value = 'live:' + element.name;
  }
  const enumerated = pseudo ? globalThis.pseudoMeasured : null;
  return {
    content: described ? generated : '',
    *[Symbol.iterator](){
      if (enumerated) enumerated.push(element.name + pseudo);
      if (!described) return;
      yield 'color';
      yield '--brand';
    },
    getPropertyValue: property => (described ? property + '=' + value : '')
  };
};
const read = probe => {
  globalThis.measured = [];
  globalThis.pseudoMeasured = [];
  globalThis.live = [];
  globalThis.pseudoReads = 0;
  globalThis.order = [];
  globalThis.sheets = 0;
  return eval(SCRIPT + '\nmeasureBaselines(documentElement, () => false);\n' + (probe || 'null'));
};
"#;

pub(crate) fn evaluate(body: &str, expression: &str) -> serde_json::Value {
    let script =
        serde_json::to_string(&crate::style_baseline::source()).expect("source is a string");
    node_eval::evaluate(
        &format!("const SCRIPT = {script};\n{DOUBLE}\n{body}"),
        expression,
    )
}
