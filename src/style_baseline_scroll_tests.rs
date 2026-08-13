//! The style baseline probe reverts every element to the user-agent origin to measure it.
//! Reverting removes the declarations that give a scroll container its range, so the engine
//! clamps the offset to zero, and restoring the style attribute does not bring it back.
//! These tests run the shipped script against a double that clamps the same way.

use crate::node_eval;

/// A document double whose scroll offsets collapse when an element's range is removed,
/// which is what a browser does when `all: revert` drops `height` and `overflow`. Its
/// elements shadow `style` with a symbol, as a custom element declaring the class field
/// `style = v` does, so a probe that reached through the instance would throw here.
const DOUBLE: &str = r#"
class Element {
  constructor(tagName, scrollable){
    this.tagName = tagName;
    this.children = [];
    this.shadowRoot = null;
    this.attributes = new Map();
    this.style = Symbol('clobbered');
    this.scrollable = scrollable;
    this.scrollLeft = 0;
    this.scrollTop = 0;
    this.modal = false;
  }
  matches(selector){ return selector === ':modal' && this.modal; }
  collapse(){ this.scrollLeft = 0; this.scrollTop = 0; this.scrollable = false; }
  appendChild(child){ this.children.push(child); return child; }
  removeChild(child){ this.children = this.children.filter(item => item !== child); }
  remove(){}
  getAttribute(name){ return this.attributes.has(name) ? this.attributes.get(name) : null; }
  setAttribute(name, value){
    this.attributes.set(name, value);
    if (name !== 'style') return;
    if (value.includes('all:revert !important')) this.collapse();
    else this.scrollable = true;
  }
  removeAttribute(name){
    this.attributes.delete(name);
    if (name === 'style') this.scrollable = true;
  }
  scrollTo(left, top){
    if (!this.scrollable) return;
    this.scrollLeft = left;
    this.scrollTop = top;
  }
}
const documentElement = new Element('HTML', true);
const panel = documentElement.appendChild(new Element('DIV', true));
panel.appendChild(new Element('DIV', false));
const head = documentElement.appendChild(new Element('HEAD', false));
globalThis.document = {
  documentElement,
  head,
  createElement: tag => new Element(tag.toUpperCase(), false)
};
globalThis.getComputedStyle = () => ({
  *[Symbol.iterator](){ yield 'color'; },
  getPropertyValue: () => 'rgb(0, 0, 0)'
});
"#;

fn measure() -> serde_json::Value {
    let source = crate::style_baseline::source();
    node_eval::evaluate(
        &format!(
            "{DOUBLE}\n{source}\npanel.scrollTop = 300;\
             \nmeasureBaselines(documentElement, () => false);"
        ),
        "[panel.scrollTop, panel.scrollLeft]",
    )
}

/// The defect: an element holding a scroll offset loses it to the measurement, so every
/// recorded offset in every capture reads zero and the value can never reach generation.
#[test]
fn restores_the_scroll_offsets_its_revert_collapsed() {
    assert_eq!(measure(), serde_json::json!([300, 0]));
}

/// The probe must put the page back completely, not merely restore what it wrote.
#[test]
fn leaves_the_style_attribute_as_it_found_it() {
    let source = crate::style_baseline::source();
    let result = node_eval::evaluate(
        &format!(
            "{DOUBLE}\n{source}\npanel.setAttribute('style', 'color: red');\
             \nmeasureBaselines(documentElement, () => false);"
        ),
        "panel.getAttribute('style')",
    );
    assert_eq!(result, serde_json::json!("color: red"));
}
