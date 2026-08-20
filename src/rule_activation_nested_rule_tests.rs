//! The subject of a rule that does not state one.
//!
//! CSS Nesting makes a style rule two things at once: a record with its own declarations,
//! and a container of further rules. The walk decided both with one test — `rule instanceof
//! CSSGroupingRule` — so a style rule was recorded and its children were never read. Nothing
//! downstream reported the loss, because a state rule that never reached the record is
//! indistinguishable from a page that authored none.
//!
//! A nested selector is relative to the rule it sits in, and the CSSOM absolutises it before
//! serialising, so `&` is always present. `&` stands for the parent list wrapped in `:is()`,
//! not for its text: the two differ in what they match, in specificity, and in whether they
//! reach pseudo-elements.

use super::{style, walk};
use serde_json::{Value, json};

fn element(path: &str, classes: Value) -> Value {
    json!({ "path": path, "classes": classes })
}

/// One parent holding one nested state rule, plus a flat control that never nested.
fn scene() -> Value {
    json!({
        "elements": [
            element("/page/tab", json!(["tab"])),
            element("/page/flat", json!(["flat"]))
        ],
        "matching": {},
        "sheets": [[
            {
                "selectorText": ".tab",
                "declarations": { "color": "#101010" },
                "rules": [style(".tab:hover", "background-color", "#b00020")]
            },
            style(".flat:hover", "background-color", "#1b5e20")
        ]]
    })
}

/// Every state record the walk made, as `target|pseudo|declaration`.
fn states(result: &Value) -> Vec<String> {
    result["stateStyles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            format!(
                "{}|{}|{}",
                entry["target"].as_str().unwrap_or(""),
                entry["pseudo"].as_str().unwrap_or(""),
                entry["declarations"].as_str().unwrap_or("")
            )
        })
        .collect()
}

/// The filed defect. The nested rule's declarations reach the same element the flat control's
/// do, so the only thing separating them is where the rule was written.
#[test]
fn a_state_rule_written_inside_its_parent_is_recorded_like_one_written_beside_it() {
    let mut scene = scene();
    scene["sheets"][0][0]["rules"] = json!([style("&:hover", "background-color", "#b00020")]);
    let recorded = states(&walk(scene));

    assert!(
        recorded.contains(&"/page/tab|:hover|background-color: #b00020;".into()),
        "a state rule authored inside its parent was never recorded: {recorded:#?}"
    );
    assert!(
        recorded.contains(&"/page/flat|:hover|background-color: #1b5e20;".into()),
        "the flat control stopped being recorded: {recorded:#?}"
    );
}

/// The discriminator against textual concatenation. Prefixing the parent's text onto
/// `&:hover` yields `.one, .two:hover`, which leaves `.one` with no state at all. Only
/// `:is(.one, .two):hover` puts the state on both, which is what the author wrote.
#[test]
fn a_nesting_selector_stands_for_the_whole_parent_list_rather_than_its_text() {
    let scene = json!({
        "elements": [
            element("/page/one", json!(["one"])),
            element("/page/two", json!(["two"]))
        ],
        "matching": {},
        "sheets": [[{
            "selectorText": ".one, .two",
            "declarations": { "color": "#101010" },
            "rules": [style("&:hover", "outline-color", "#0d47a1")]
        }]]
    });
    let recorded = states(&walk(scene));

    assert!(
        recorded.contains(&"/page/one|:hover|outline-color: #0d47a1;".into()),
        "the first member of the parent list lost its state: {recorded:#?}"
    );
    assert!(
        recorded.contains(&"/page/two|:hover|outline-color: #0d47a1;".into()),
        "the last member of the parent list lost its state: {recorded:#?}"
    );
}

/// `&` is not required to lead. The bundle writes `.other &` and `&:where(…)`, so a repair
/// keyed on the first character of the selector answers neither.
#[test]
fn a_nesting_selector_is_composed_wherever_in_the_member_it_stands() {
    let scene = json!({
        "elements": [
            element("/page/wrap", json!(["wrap"])),
            element("/page/wrap/pill", json!(["pill"]))
        ],
        "matching": {},
        "sheets": [[{
            "selectorText": ".pill",
            "declarations": { "color": "#101010" },
            "rules": [style(".wrap &:hover", "border-color", "#4a148c")]
        }]]
    });
    let recorded = states(&walk(scene));

    assert!(
        recorded.contains(&"/page/wrap/pill|:hover|border-color: #4a148c;".into()),
        "a nesting selector that was not first was not composed: {recorded:#?}"
    );
}

/// Nesting stacks, and the state can sit on a level above the element the rule paints. This
/// is the shape the whole walk has to carry: two levels, a combinator, and a state to the
/// left of it. Composing only the innermost level would leave the holder unresolved.
#[test]
fn nesting_composes_through_more_than_one_level() {
    let scene = json!({
        "elements": [
            element("/page/row", json!(["row"])),
            element("/page/row/cell", json!(["cell"]))
        ],
        "matching": {},
        "sheets": [[{
            "selectorText": ".row",
            "declarations": { "color": "#101010" },
            "rules": [{
                "selectorText": "&:hover",
                "declarations": { "color": "#202020" },
                "rules": [style("& > .cell", "background-color", "#004d40")]
            }]
        }]]
    });
    let result = walk(scene);
    let held = result["stateStyles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["declarations"]
                .as_str()
                .unwrap_or("")
                .contains("#004d40")
        })
        .cloned()
        .unwrap_or(Value::Null);

    assert_eq!(
        held["target"], "/page/row/cell",
        "a rule nested two levels deep did not reach the element it paints: {held:#?}"
    );
    assert_eq!(
        held["scope"], "/page/row",
        "the state holder two levels up was not resolved: {held:#?}"
    );
    assert_eq!(
        held["relation"], "parent",
        "the combinator the author wrote between the levels was not kept: {held:#?}"
    );
    assert_eq!(
        held["pseudo"], ":hover",
        "the state itself was lost: {held:#?}"
    );
}

#[path = "rule_activation_nested_shape_tests.rs"]
mod shape;
