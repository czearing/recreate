//! What a baseline is measured against when the page has said the same thing more forcefully.
//!
//! Reaching the scope a thing is in is only half of measuring it. The other half is being in
//! force once there, and a rule is in force only against the declarations it outranks. The
//! element half of the probe carries its rollback on a style attribute, which is not a selector
//! and so is never weighed at all; the pseudo half has no node to carry anything, so its
//! rollback is a rule, and a rule that wins by selecting harder can always be beaten by a page
//! that selects harder still. An important author declaration on a pseudo-element beats every
//! universal rule there is, so the baseline reads back as the live value, the difference is
//! empty, and the declaration is pruned as unauthored - silently, because a box that carries
//! content is still recorded for the content, and a read that returned values is not a failure.
//!
//! These tests are a 2x2 over one property: element against pseudo-element, normal importance
//! against important. Three of the four cells pass before the repair and are here to fail after
//! a repair that trades them away.

use crate::style_baseline_double::evaluate;

/// The page: one element per cell of the 2x2. Each carries an authored `::before`; the important
/// pair additionally declare that the page said it with `!important`.
const CELLS: &str = r#"
const cell = id => {
  const element = body.add(new Element('P'));
  element.setAttribute('id', id);
  globalThis.content.set('P#' + id + '::before', '"x"');
  globalThis.pseudoAuthored.add('P#' + id + '::before');
  globalThis[id] = element;
  return element;
};
cell('normal');
cell('important');
globalThis.pseudoImportant.add('P#important::before');
"#;

fn measured(probe: &str) -> serde_json::Value {
    evaluate(
        &format!("{CELLS}\nread();"),
        &format!("eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\\n{probe}')"),
    )
}

/// The filed case, asserted on the node record rather than on emitted CSS, so that no later
/// change to the emitter can make an acquisition loss look repaired.
#[test]
fn an_important_pseudo_declaration_reaches_the_record_the_normal_one_reaches() {
    let seen = measured(
        "[recreatePseudos(normal)[\"::before\"].style, recreatePseudos(important)[\"::before\"].style]",
    );
    assert_eq!(
        seen,
        serde_json::json!([
            { "color": "color=pseudo-authored:P#normal::before" },
            { "color": "color=pseudo-authored:P#important::before" }
        ]),
        "a declaration the page marked important is measured against its own live value, so the \
         difference is empty and it is pruned as though the page had never authored it"
    );
}

/// The whole walk, stated as an invariant rather than per element: importance is the page's to
/// choose, so no baseline may vary with it. A repair that reaches the filed element by naming
/// importance, or one that covers only the selectors this fixture happens to write, fails here.
#[test]
fn no_baseline_the_walk_records_depends_on_how_forcefully_the_page_said_it() {
    let unreached = measured(
        "globalThis.walked.flatMap(element => generatedBoxTests() \
         .flatMap(([name]) => Object.values(pseudoBaselineOf(element, name)) \
         .filter(value => value.includes(\"authored\")).map(() => element.name + name))).sort()",
    );
    assert_eq!(
        unreached,
        serde_json::json!([]),
        "a baseline the page outranked is the live value, so everything on it is pruned"
    );
}

/// The control row: the element half already wins without being weighed, and must still do so.
/// A repair that moved the element rollback into the same carrier as the pseudo one would put it
/// back into a contest it currently never enters.
#[test]
fn the_element_half_still_measures_both_importances_the_same_way() {
    let seen = measured("[baselineOf(normal)[\"color\"], baselineOf(important)[\"color\"]]");
    assert_eq!(
        seen,
        serde_json::json!(["color=revert:P#normal", "color=revert:P#important"]),
        "the style attribute is not a selector, so nothing the page declares can outrank it"
    );
}

/// Losing the contest and never entering it are indistinguishable from the emitted output, and
/// only one of them is reported. This pins that the box survives either way, which is why the
/// loss is silent and why the record is the only place worth asserting on.
#[test]
fn a_box_whose_declarations_were_all_outranked_is_still_recorded_by_its_content() {
    let seen = measured("recreatePseudos(important)[\"::before\"].content");
    assert_eq!(
        seen,
        serde_json::json!("\"x\""),
        "the box is kept by its content, so nothing counts the lost declarations as declined"
    );
}
