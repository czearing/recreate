//! A baseline reading that no consumer can reach is work the capture pays for and throws away.
//! The probe enumerates every element four times - live, `::before`, `::after` and the reverted
//! element - but a pseudo-element is only ever recorded when it generates a box, and the live
//! enumeration is the same one every consumer then repeats. These tests pin both prunings to the
//! condition that makes them safe rather than to a count: a pseudo baseline is skipped exactly
//! when the value the consumer tests says it would be discarded, and the handed-over live map is
//! the one the probe took, so a pruning that reached further would show up as a missing or wrong
//! recorded value rather than as a fast test.

use crate::node_eval;

/// The double reports a value built from the inputs a reverted element actually depends on and
/// records every enumeration it serves, separated by whether it was live, reverted or a pseudo.
/// `content` is authored per element and per pseudo name so a test can move exactly one of them.
const DOUBLE: &str = r#"
globalThis.content = new Map();
globalThis.mark = stage => {
  if (globalThis.order[globalThis.order.length - 1] !== stage) globalThis.order.push(stage);
};
class Style {
  constructor(element){ this.element = element; }
  setProperty(name, value){
    if (name === 'all' && value === 'revert') { this.element.reverted = true; globalThis.mark('revert'); }
  }
}
class Element {
  constructor(tagName){
    this.tagName = tagName;
    this.children = [];
    this.shadowRoot = null;
    this.attributes = new Map();
    this.style = new Style(this);
    this.reverted = false;
    this.parent = null;
  }
  add(child){ child.parent = this; this.children.push(child); return child; }
  getAttribute(name){ return this.attributes.has(name) ? this.attributes.get(name) : null; }
  setAttribute(name, value){
    this.attributes.set(name, value);
    if (name === 'style') { this.reverted = false; globalThis.mark('restore'); }
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
const marked = body.add(new Element('P'));
marked.setAttribute('id', 'marked');
globalThis.marked = marked;
globalThis.plain = plain;
globalThis.body = body;
head.appendChild = child => { head.add(child); globalThis.sheets += 1; };
globalThis.document = {
  documentElement,
  head,
  createElement: tag => {
    const made = new Element(tag.toUpperCase());
    made.remove = () => { head.children = head.children.filter(item => item !== made); };
    return made;
  }
};
globalThis.getComputedStyle = (element, pseudo) => {
  const generated = globalThis.content.get(element.name + (pseudo || '')) || 'none';
  let value;
  if (pseudo) {
    if (element.reverted) throw new Error('pseudo read while the element was reverted');
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
    content: generated,
    *[Symbol.iterator](){
      if (enumerated) enumerated.push(element.name + pseudo);
      yield 'color';
      yield '--brand';
    },
    getPropertyValue: property => property + '=' + value
  };
};
const read = probe => {
  globalThis.measured = [];
  globalThis.pseudoMeasured = [];
  globalThis.live = [];
  globalThis.order = [];
  globalThis.sheets = 0;
  return eval(SCRIPT + '\nmeasureBaselines(documentElement, () => false);\n' + (probe || 'null'));
};
"#;

fn evaluate(body: &str, expression: &str) -> serde_json::Value {
    let script = serde_json::to_string(crate::style_baseline::SOURCE).expect("source is a string");
    node_eval::evaluate(
        &format!("const SCRIPT = {script};\n{DOUBLE}\n{body}"),
        expression,
    )
}

/// The defect: every element paid for two full pseudo-element enumerations under a revert sheet
/// even though the recording is discarded unless the pseudo generates content. Almost no element
/// on a page does, so this is where the enumeration count lives.
#[test]
fn measures_no_pseudo_baseline_when_nothing_generates_content() {
    let seen = evaluate("read();", "[globalThis.pseudoMeasured, globalThis.sheets]");
    assert_eq!(seen, serde_json::json!([[], 0]));
}

/// The consumer asks for one pseudo name at a time and only after testing that name's content,
/// so measuring the sibling name records a baseline nothing can read.
#[test]
fn measures_only_the_pseudo_name_that_generates_content() {
    let seen = evaluate(
        "globalThis.content.set('P#marked::after', '\"x\"'); read();",
        "[globalThis.pseudoMeasured, globalThis.sheets]",
    );
    assert_eq!(seen, serde_json::json!([["P#marked::after"], 1]));
}

/// Pruning must not lose the reading. A pseudo that does generate content still needs a baseline
/// measured under the revert sheet, or every one of its declarations would be published as
/// authored when the user-agent supplied it.
#[test]
fn still_records_the_baseline_of_a_pseudo_that_generates_content() {
    let value = evaluate(
        "globalThis.content.set('P#marked::before', 'counter(step)'); read();",
        "eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\
         \\npseudoBaselineOf(marked, \"::before\")')",
    );
    assert_eq!(
        value,
        serde_json::json!({ "color": "color=pseudo:P#marked::before" })
    );
}

/// The reading this pruning is allowed to skip is decided by the value the consumer tests, so it
/// has to be taken from the same page the consumer sees: the restored one, after every style
/// attribute is back. Reverting an element drops its `animation` and `transition` declarations
/// and restoring them starts both over, so a value read before the pass describes a page the
/// capture never publishes. This is the defect that a byte comparison of the corpus caught while
/// a timing budget passed.
#[test]
fn tests_generated_content_only_after_the_page_is_restored() {
    let order = evaluate(
        "globalThis.content.set('P#marked::before', '\"x\"'); read();",
        "[globalThis.pseudoMeasured, globalThis.order.slice(-2)]",
    );
    assert_eq!(
        order,
        serde_json::json!([["P#marked::before"], ["restore", "pseudo"]])
    );
}

/// `all` does not reach custom properties, so a reverted element reports the same ones it reports
/// live and every comparison against the baseline already discards them. Enumerating them is the
/// largest variable-sized part of the read, because a design system declares its whole palette on
/// one inherited root.
#[test]
fn enumerates_no_custom_property_in_the_baseline() {
    let value = evaluate(
        "read();",
        "eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\
         \\nObject.keys(baselineOf(marked))')",
    );
    assert_eq!(value, serde_json::json!(["color"]));
}

/// Inheritance is one-way, so a level is reverted only after every level above it was measured
/// and put back. A pruning that reordered the walk would let a child inherit a reverted parent.
#[test]
fn reverts_parents_before_children_and_restores_every_element() {
    let value = evaluate(
        "read();",
        "[globalThis.measured, [documentElement, body, marked].map(node => node.reverted)]",
    );
    assert_eq!(
        value,
        serde_json::json!([
            ["HTML", "HEAD", "BODY", "P", "P#marked"],
            [false, false, false]
        ])
    );
}
