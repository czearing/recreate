//! A condition nested inside another, and a condition that is no condition at all.
//!
//! A capture serialises a sheet linked with `media="all"` as `@media all{...}`, so a page's
//! real breakpoints arrive one group inside another and the identity condition arrives around
//! everything else. Both readings are asserted here against the shipped stages.

use super::restore_unconditional;
use crate::generate::authored_conditions::rules;
use crate::generate::authored_css_index::Index;
use crate::generate::selector_scope::Scope;
use crate::model::{Attributes, Node, Rect, Styles};
use std::collections::{BTreeMap, BTreeSet};

fn node(classes: &str, style: &[(&str, &str)]) -> Node {
    Node {
        path: "html>body>div:nth-of-type(1)".into(),
        parent: None,
        tag: "div".into(),
        attributes: Attributes::from([("class".into(), classes.into())]),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
        },
        style: style
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
        ..Default::default()
    }
}

fn restored(node: &Node, captured: &[String]) -> Styles {
    let mut styles = node.style.clone();
    restore_unconditional(&mut styles, node, &Index::new(captured));
    styles
}

fn emitted(node: &Node, captured: &[String]) -> Vec<String> {
    let nodes = vec![node.clone()];
    let classes = BTreeMap::from([(node.path.clone(), "card".to_string())]);
    let scope = Scope::new(&nodes, &classes, "r");
    let mut compounds = BTreeSet::new();
    rules(&nodes[0], &scope, captured, &mut compounds)
        .iter()
        .map(crate::generate::authored_conditions::Emitted::text)
        .collect()
}

/// The sheet-level wrapper a capture writes around a `media="all"` stylesheet, holding the
/// page's real breakpoint. A reader that stops at the outer group sees a selector list spelled
/// `@media (min-width: 600px)`, matches it against nothing, and reports the page as carrying no
/// conditional rule at all — which is the whole population on a real site.
fn nested() -> Vec<String> {
    vec![
        "@media all{.card { background-color: rgb(255, 0, 0); }}".into(),
        "@media all{@media (min-width: 600px){.card { background-color: rgb(0, 0, 255); }}}".into(),
    ]
}

/// The same page with the base arm authored outside the wrapper, so the unconditional cascade
/// owns it and the base rule is the only thing that can publish it.
fn nested_with_unconditional_base() -> Vec<String> {
    vec![
        ".card { background-color: rgb(255, 0, 0); }".into(),
        "@media all{@media (min-width: 600px){.card { background-color: rgb(0, 0, 255); }}}".into(),
    ]
}

#[test]
fn restores_the_base_arm_of_a_breakpoint_nested_inside_the_sheet_wrapper() {
    let node = node("card", &[("background-color", "rgb(0, 0, 255)")]);
    let styles = restored(&node, &nested_with_unconditional_base());

    assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
}

#[test]
fn re_emits_a_nested_breakpoint_inside_every_condition_it_was_authored_in() {
    let node = node("card", &[("background-color", "rgb(0, 0, 255)")]);
    let emitted = emitted(&node, &nested());

    assert!(
        emitted.contains(
            &"@media all{@media (min-width: 600px){.card{background-color: rgb(0, 0, 255);}}}"
                .to_string()
        ),
        "{emitted:?}"
    );
}

/// Withdrawal and re-emission read one walk, so a declaration taken out of the base rule is
/// always put back under the condition that owns it. Where both arms were authored inside the
/// sheet wrapper the base rule publishes neither and the emitted conditions publish both, which
/// is the only reading that paints each arm on its own side of the breakpoint.
#[test]
fn puts_back_under_its_condition_every_declaration_it_withdraws() {
    let node = node("card", &[("background-color", "rgb(0, 0, 255)")]);
    let restored = restored(&node, &nested());
    let emitted = emitted(&node, &nested()).join("");

    assert!(!restored.contains_key("background-color"), "{restored:?}");
    assert!(
        emitted.contains("@media all{.card{background-color: rgb(255, 0, 0);}}"),
        "{emitted}"
    );
    assert!(
        emitted.contains("@media (min-width: 600px){.card{background-color: rgb(0, 0, 255);}}"),
        "{emitted}"
    );
}

/// `all` is the media type Media Queries 4 defines as matching every device, so `@media all`
/// holds wherever the base rule does. There is no arm below it to restore, and withdrawing
/// against it would delete the only one the author wrote.
#[test]
fn keeps_a_declaration_guarded_only_by_the_identity_condition() {
    let node = node(
        "card",
        &[("position", "absolute"), ("left", "18px"), ("top", "12px")],
    );
    let captured =
        vec!["@media all{.card { position: absolute; left: 18px; top: 12px; }}".to_string()];
    let styles = restored(&node, &captured);

    assert_eq!(styles["position"], "absolute");
    assert_eq!(styles["left"], "18px");
    assert_eq!(styles["top"], "12px");
}

/// The identity condition is recognised by what it says, not by where it sits: the same words
/// around a real breakpoint must not exempt that breakpoint from withdrawal.
#[test]
fn does_not_exempt_a_real_breakpoint_merely_wrapped_in_the_identity_condition() {
    let node = node("card", &[("background-color", "rgb(0, 0, 255)")]);
    let styles = restored(&node, &nested_with_unconditional_base());

    assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
}

/// A media type that is not `all` still has a false branch — `print` is false on screen — so
/// nothing here may generalise the exemption from the identity condition to media types.
#[test]
fn withdraws_against_a_media_type_that_can_be_false() {
    let node = node("card", &[("background-color", "rgb(0, 0, 255)")]);
    let captured = vec![
        ".card { background-color: rgb(255, 0, 0); }".to_string(),
        "@media print{.card { background-color: rgb(0, 0, 255); }}".to_string(),
    ];

    assert_eq!(restored(&node, &captured)["background-color"], "rgb(255, 0, 0)");
}
