//! Which element a state rule is recorded against, and what fires it.
//!
//! A state rule is captured by matching its selector against the live page, so every question
//! this stage asks is a question about the selector's structure. It used to answer them by
//! cutting the text apart — `split(',')` for the list, a regex for the states, a slice for the
//! element holding them — and each cut lands mid-construct on a selector whose commas are
//! nested. The fragment that comes out is usually still a selector, so the failure is not a
//! parse error anyone sees: the rule matches a population the author never named, or throws
//! where the throw is swallowed and nothing is recorded at all.
//!
//! These tests drive the real capture over a scripted CSSOM and assert on the record, because
//! a rule that never reaches the record cannot be rescued by any later stage.

use super::{style, walk};
use serde_json::{Value, json};

fn element(path: &str, class: &str) -> Value {
    json!({ "path": path, "classes": [class], "computed": {} })
}

fn disabled(path: &str, class: &str) -> Value {
    json!({
        "path": path,
        "classes": [class],
        "computed": {},
        "attributes": { "data-disabled": "" }
    })
}

/// Five state rules over one document, in the shapes a shipped bundle authors them in.
///
/// `.wrap` and `.wrapOff` differ only in whether the input inside them is disabled, which is
/// what the nested `:not(...)` list decides — so a reader that mishandles the comma inside it
/// cannot pass by recording both or neither.
pub fn scene() -> Value {
    let sheet = json!([
        style(
            ".wrap:has(.inner:not([data-disabled],[aria-invalid=true]):focus-visible)",
            "border-color",
            "#242424"
        ),
        style(".plain:focus-visible", "border-color", "#0f6cbd"),
        style(".row:hover .badge", "color", "#c50f1f"),
        style(".card:has(.btn:hover)", "border-color", "#107c10"),
        style(
            ".ring:where(:focus-visible,[data-activedescendant-focusvisible])",
            "border-color",
            "#8764b8"
        )
    ]);
    json!({
        "elements": [
            element("/main/wrap", "wrap"),
            element("/main/wrap/inner", "inner"),
            element("/main/wrapOff", "wrap"),
            disabled("/main/wrapOff/inner", "inner"),
            element("/main/plain", "plain"),
            element("/main/row", "row"),
            element("/main/row/badge", "badge"),
            element("/main/card", "card"),
            element("/main/card/btn", "btn"),
            element("/main/ring", "ring")
        ],
        "matching": {},
        "sheets": [sheet]
    })
}

/// Every record the walk produced, as `(target, scope, relation, pseudo)`.
pub fn recorded(scene: Value) -> Vec<(String, String, String, String)> {
    walk(scene)["stateStyles"]
        .as_array()
        .expect("the walk records state styles")
        .iter()
        .map(|entry| {
            let text = |key: &str| entry[key].as_str().unwrap_or_default().to_string();
            (
                text("target"),
                text("scope"),
                text("relation"),
                text("pseudo"),
            )
        })
        .collect()
}

/// The filed defect. Two of the five rules carry a comma inside a functional pseudo-class,
/// and both used to leave no record at all — while the three without one were recorded, which
/// is what made the loss look like nothing was wrong.
#[test]
fn every_authored_state_rule_reaches_the_record() {
    let records = recorded(scene());
    assert_eq!(
        records.len(),
        5,
        "five rules were authored, these were recorded: {records:#?}"
    );
    assert!(
        records.iter().any(|(target, ..)| target == "/main/ring"),
        "the rule whose state sits behind a nested comma was recorded: {records:#?}"
    );
}

/// A state held inside the element the rule styles is a different relation from a state held
/// above it, and CSS has one construct for each. Recording only the pair leaves them
/// indistinguishable, and the emitter then stamps the state on the container, so the style
/// spreads over the whole card instead of answering the button inside it.
#[test]
fn a_state_held_inside_the_subject_is_recorded_as_contained() {
    let records = recorded(scene());
    assert!(
        records.contains(&(
            "/main/card".into(),
            "/main/card/btn".into(),
            "contained".into(),
            ":hover".into()
        )),
        "the button holds the hover and the card takes the style: {records:#?}"
    );
    assert!(
        records.contains(&(
            "/main/wrap".into(),
            "/main/wrap/inner".into(),
            "contained".into(),
            ":focus-visible".into()
        )),
        "the input holds the focus and the wrapper takes the style: {records:#?}"
    );
}

/// The two relations the pair already expressed must be untouched: a state on the subject
/// itself has no scope, and a state on an ancestor keeps the ancestor as its scope.
#[test]
fn a_state_on_the_subject_or_above_it_is_recorded_exactly_as_before() {
    let records = recorded(scene());
    assert!(records.contains(&(
        "/main/plain".into(),
        String::new(),
        "ancestor".into(),
        ":focus-visible".into()
    )));
    assert!(records.contains(&(
        "/main/row/badge".into(),
        "/main/row".into(),
        "ancestor".into(),
        ":hover".into()
    )));
}

/// The nested list is a test, not decoration. `:not([data-disabled],[aria-invalid=true])`
/// excludes the disabled input, so the wrapper around it must not be recorded — which a
/// reader that cut the list at its comma cannot get right in either direction.
#[test]
fn a_nested_list_still_decides_which_elements_the_rule_reaches() {
    let records = recorded(scene());
    assert!(
        !records.iter().any(|(target, ..)| target == "/main/wrapOff"),
        "the disabled input's wrapper was excluded by the nested list: {records:#?}"
    );
}
