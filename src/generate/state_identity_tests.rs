//! What a generated class has to encode before two elements may share it.
//!
//! A class is minted per identity and its rules are written once, so two elements sharing a
//! class share every rule the class carries — including the ones that apply only while the
//! page is in some state. An identity built from the resting style alone gives one class to
//! two elements that rest alike and answer focus differently, and the emitter then writes two
//! rules with the same selector, of which only the last survives. The element whose rule lost
//! silently takes the other's colour.
//!
//! These tests call the owner of identity directly, so they fail for the reason they name.

use super::css_pseudo_identity_tests::span;
use super::css_values::responsive_signatures_for;
use crate::model::{PageState, Relation, Specification, StateStyle, Viewport};
use std::collections::BTreeMap;

fn state_style(target: &str, scope: Option<&str>, declarations: &str) -> StateStyle {
    StateStyle {
        target: target.into(),
        scope: scope.map(str::to_string),
        relation: Relation::Ancestor,
        pseudo: Some(":focus-visible".into()),
        target_pseudo: None,
        media: None,
        declarations: declarations.into(),
    }
}

/// The paths of the three spans each scene is built from.
fn paths() -> Vec<String> {
    [span(1), span(2), span(3)]
        .iter()
        .map(|node| node.path.clone())
        .collect()
}

/// The identity of each node, as the class minter reads it.
fn signatures(styles: Vec<StateStyle>) -> Vec<String> {
    let nodes = vec![span(1), span(2), span(3)];
    let paths: Vec<String> = nodes.iter().map(|node| node.path.clone()).collect();
    let specification = Specification {
        schema_version: 1,
        requested_url: "https://example.com".into(),
        captured_url: "https://example.com".into(),
        states: vec![PageState {
            url: "https://example.com".into(),
            title: "Example".into(),
            viewport: Viewport {
                width: 1920,
                height: 1080,
                dpr: 1.0,
            },
            nodes,
            state_styles: styles,
            ..Default::default()
        }],
        interactions: Vec::new(),
        transitions: Vec::new(),
    };
    let signatures = responsive_signatures_for(&specification, None, &BTreeMap::new());
    paths.iter().map(|path| signatures[path].clone()).collect()
}

/// The defect this guards. The two spans rest identically, so nothing in the resting style can
/// tell them apart; only the rule each receives under focus can.
#[test]
fn two_elements_that_answer_a_state_differently_are_two_elements() {
    let paths = paths();
    let signatures = signatures(vec![
        state_style(&paths[0], None, "border-color: #0f6cbd;"),
        state_style(&paths[1], None, "border-color: #8764b8;"),
    ]);
    assert_ne!(
        signatures[0], signatures[1],
        "two elements with different focus rules were given one identity"
    );
}

/// A rule names two elements, so moving the one that holds the state changes what is emitted
/// even though the element being styled receives the same declarations.
#[test]
fn an_element_styled_from_elsewhere_is_identified_by_where_from() {
    let paths = paths();
    let held_above = signatures(vec![state_style(&paths[0], Some(&paths[1]), "color: red;")]);
    let held_alone = signatures(vec![state_style(&paths[0], None, "color: red;")]);
    assert_ne!(
        held_above[0], held_alone[0],
        "a rule fired from an ancestor was identified as one fired by the element itself"
    );
}

/// A state rule is not carried by the element it styles alone: the element holding the state
/// appears in the selector too, so it cannot be collapsed onto an element that holds nothing.
/// The rule here styles a third element, so nothing about what these two receive can tell them
/// apart — only which of them fires it.
#[test]
fn the_element_holding_a_state_carries_the_rule_as_well() {
    let paths = paths();
    let signatures = signatures(vec![state_style(&paths[2], Some(&paths[0]), "color: red;")]);
    assert_ne!(
        signatures[0], signatures[1],
        "the element whose focus fires a rule was collapsed onto one that fires nothing"
    );
}

/// The inverse guard. Identity must not be traded for a stylesheet with one class per element:
/// two elements receiving the same rule from equivalent places still share one.
#[test]
fn two_elements_receiving_the_same_rule_still_share_one_identity() {
    let paths = paths();
    let signatures = signatures(vec![
        state_style(&paths[0], None, "border-color: #0f6cbd;"),
        state_style(&paths[1], None, "border-color: #0f6cbd;"),
    ]);
    assert_eq!(
        signatures[0], signatures[1],
        "two elements answering focus alike were given two identities"
    );
}

/// An element no state rule names must keep exactly the identity it had before state rules were
/// folded in at all. Folding "nothing" into every element is equally correct and renames every
/// class on every page, which turns any future diff of generated output into noise.
#[test]
fn an_element_no_state_rule_names_keeps_its_resting_identity() {
    let paths = paths();
    let untouched = signatures(vec![])[1].clone();
    let alongside = signatures(vec![state_style(&paths[0], None, "color: red;")])[1].clone();
    assert_eq!(
        untouched, alongside,
        "an element no state rule mentions was renamed by a rule about another element"
    );
}
