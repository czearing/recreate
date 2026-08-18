//! The base rule must publish the arm that holds when no document-answered condition does.
//!
//! Every case drives the shipped stage against a captured style plus the authored rule text
//! the capture carries, which is what a real capture hands the emitter.

use super::restore_unconditional;
use crate::generate::authored_css_index::Index;
use crate::model::{Attributes, Node, Rect, Styles};

fn node(classes: &str, style: &[(&str, &str)]) -> Node {
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: String::new(),
        parent: None,
        tag: "div".into(),
        text: String::new(),
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

/// The engine's own answer, as the capture records it: the properties whose computed value
/// moved when the rules the recreation re-emits under a condition were withdrawn.
///
/// Stated by each case rather than derived here, because deriving it would mean resolving the
/// cascade from the same authored text the stage under test reads — which is the proxy this
/// evidence replaced. A case that states none is a page where no condition decided anything.
fn decided(mut node: Node, properties: &[&str]) -> Node {
    node.condition_decided = properties.iter().map(|name| (*name).to_string()).collect();
    node
}

/// The stage as the emitters run it: the captured style is what the base rule would say,
/// and the authored rules are the ones the capture recorded alongside it.
fn restored(node: &Node, captured: &[String]) -> Styles {
    let mut styles = node.style.clone();
    restore_unconditional(&mut styles, node, &Index::new(captured));
    styles
}

fn scene(subject: &str, condition: &str, over: &str) -> Vec<String> {
    vec![
        format!(".{subject} {{ height: 40px; background-color: rgb(255, 0, 0); }}"),
        format!("@media {condition}{{.{subject} {{ background-color: {over}; }}}}"),
    ]
}

/// The defect. Every sampled viewport sat above the breakpoint, so the base rule published
/// the override and the arm below it existed nowhere in the output.
#[test]
fn publishes_the_arm_below_the_breakpoint_rather_than_the_measured_override() {
    let node = decided(
        node(
            "unsampled",
            &[("background-color", "rgb(0, 0, 255)"), ("height", "40px")],
        ),
        &["background-color"],
    );
    let styles = restored(
        &node,
        &scene("unsampled", "(min-width: 600px)", "rgb(0, 0, 255)"),
    );

    assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
    assert_eq!(styles["height"], "40px");
}

/// The breakpoint the sweep does visit fails identically, so no number of sample widths is
/// the repair. Same assertion, breakpoint moved onto a sampled width.
#[test]
fn repairs_a_breakpoint_that_a_sampled_width_sits_exactly_on() {
    let node = decided(
        node("sampled", &[("background-color", "rgb(0, 255, 0)")]),
        &["background-color"],
    );
    let styles = restored(
        &node,
        &scene("sampled", "(min-width: 768px)", "rgb(0, 255, 0)"),
    );

    assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
}

/// The positive control. An element no condition names keeps every measured value, so the
/// stage cannot be passing the two above by emptying the map.
#[test]
fn leaves_an_element_no_condition_names_untouched() {
    let node = node("nomedia", &[("background-color", "rgb(255, 0, 0)")]);
    let styles = restored(
        &node,
        &scene("unsampled", "(min-width: 600px)", "rgb(0, 0, 255)"),
    );

    assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
}

/// Below its breakpoint the element takes its inherited or initial value, which the
/// recreation re-produces by saying nothing. Substituting a value it never authored, or
/// keeping the override, would both paint something the source does not.
#[test]
fn drops_a_property_the_unconditional_cascade_never_declared() {
    let node = decided(node("only", &[("color", "rgb(0, 0, 255)")]), &["color"]);
    let captured = vec!["@media (min-width: 600px){.only { color: rgb(0, 0, 255); }}".into()];
    let styles = restored(&node, &captured);

    assert!(!styles.contains_key("color"));
}

/// The stage is only a repair if the emitters actually run it. `authored_css::normalize` is
/// the seam the base rule and every viewport band share, so the repair is asserted through
/// that entry point and not only against the function that performs it.
#[test]
fn reaches_the_node_through_the_stage_both_emitters_call() {
    let node = decided(
        node(
            "unsampled",
            &[("background-color", "rgb(0, 0, 255)"), ("height", "40px")],
        ),
        &["background-color"],
    );
    let mut styles = node.style.clone();
    crate::generate::authored_css::normalize(
        &mut styles,
        &node,
        &scene("unsampled", "(min-width: 600px)", "rgb(0, 0, 255)"),
    );

    assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
    assert_eq!(styles["height"], "40px");
}

#[path = "authored_conditions_base_arm_guard_tests.rs"]
mod guards;

#[path = "authored_conditions_base_arm_nesting_tests.rs"]
mod nesting;

#[path = "authored_conditions_base_arm_shorthand_tests.rs"]
mod shorthand;

#[path = "authored_conditions_base_arm_divided_tests.rs"]
mod divided;
