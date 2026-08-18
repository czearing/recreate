//! A conditional override the author spelled as a shorthand still owes its base arm.
//!
//! A capture enumerates longhands, so `background` names no key in a sampled style. Every
//! case here authors the shorthand and asserts on the longhand the capture actually holds.

use super::{decided, restore_unconditional};
use crate::generate::authored_css_index::Index;
use crate::model::{Attributes, Node, Rect, Styles};

fn node(classes: &str, style: &[(&str, &str)]) -> Node {
    Node {
        path: String::new(),
        tag: "p".into(),
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

/// The scene's own sheet: one base arm shared by both cards, and two conditionals of
/// identical shape differing only in who answers them.
fn scene() -> Vec<String> {
    vec![
        ".cq-card { background: rgb(255, 0, 0); padding: 24px; }".into(),
        ".mq-card { background: rgb(255, 0, 0); padding: 24px; }".into(),
        "@container cardwrap (min-width: 500px){.cq-card { background: rgb(0, 255, 0); }}".into(),
        "@media (min-width: 500px){.mq-card { background: rgb(0, 0, 255); }}".into(),
    ]
}

/// The filed defect. The instance whose container answered yes had the override baked as
/// its unconditional value, and the `@container` rule then restated it, so the arm below
/// the threshold existed nowhere and shrinking the container could never reach it.
#[test]
fn publishes_the_base_arm_of_an_override_a_container_answered_yes() {
    let vorpal = decided(
        node(
            "cq-card",
            &[
                ("background-color", "rgb(0, 255, 0)"),
                ("padding-top", "24px"),
            ],
        ),
        &["background-color"],
    );

    let styles = restored(&vorpal, &scene());

    assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
    assert_eq!(styles["padding-top"], "24px");
}

/// The same assertion with only the conditional kind changed. It fails today for the same
/// reason, so the kind is not what decides and no `@container` branch can be the repair.
#[test]
fn publishes_the_base_arm_of_an_override_the_viewport_answered_yes() {
    let quillow = decided(
        node("mq-card", &[("background-color", "rgb(0, 0, 255)")]),
        &["background-color"],
    );

    let styles = restored(&quillow, &scene());

    assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
}

/// The instance whose container answered no already measured the base arm. Its conditional
/// declaration disagrees with the sample, so the condition was false and nothing is owed.
#[test]
fn leaves_the_instance_whose_container_answered_no_untouched() {
    let brimsel = node("cq-card", &[("background-color", "rgb(255, 0, 0)")]);

    let styles = restored(&brimsel, &scene());

    assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
}

/// A shorthand names its longhands by prefix, which over-answers: `border` prefixes
/// `border-radius` and does not set it. The engine's own answer is what refuses the pair — it
/// reports the width as condition-decided and the corner as untouched — so a longer list is
/// never what keeps this right.
#[test]
fn refuses_a_prefixed_property_the_shorthand_does_not_set() {
    let node = decided(
        node(
            "card",
            &[("border-radius", "4px"), ("border-top-width", "8px")],
        ),
        &["border-top-width"],
    );
    let captured = vec![
        ".card { border: 2px; border-radius: 4px; }".into(),
        "@media (min-width: 500px){.card { border: 8px; }}".into(),
    ];

    let styles = restored(&node, &captured);

    assert_eq!(styles["border-radius"], "4px");
    assert_eq!(styles["border-top-width"], "2px");
}

/// The hazard the repair itself introduces. A base arm spelled with several components is
/// divided between its longhands by a grammar this stage does not read, so the measured
/// value stands rather than being deleted for want of a replacement.
#[test]
fn keeps_the_measured_value_when_the_base_arm_divides_between_longhands() {
    let node = decided(node("card", &[("padding-top", "40px")]), &["padding-top"]);
    let captured = vec![
        ".card { padding: 24px 8px; }".into(),
        "@media (min-width: 500px){.card { padding: 40px; }}".into(),
    ];

    let styles = restored(&node, &captured);

    assert_eq!(styles["padding-top"], "40px");
}

/// A conditional value the artifact recorded no division for still names its longhands, and
/// which of them a condition decided is not this stage's to work out — so the override needs
/// no decoding at all. The base arm does need one, and `24px` has a single component, so the
/// replacement is exact. Before the engine answered the first half, this published `40px` as
/// the element's unconditional padding, which is the filed defect in shorthand clothing.
#[test]
fn withdraws_to_a_base_arm_it_can_divide_from_an_override_it_cannot() {
    let node = decided(node("card", &[("padding-top", "40px")]), &["padding-top"]);
    let captured = vec![
        ".card { padding: 24px; }".into(),
        "@media (min-width: 500px){.card { padding: 40px 8px; }}".into(),
    ];

    let styles = restored(&node, &captured);

    assert_eq!(styles["padding-top"], "24px");
}

/// A family whose longhands CSS renamed rather than prefixed. Nothing about `row-gap`
/// begins with `gap`, so only the rename table reaches it.
#[test]
fn publishes_the_base_arm_of_a_shorthand_whose_longhands_were_renamed() {
    let node = decided(node("row", &[("row-gap", "32px")]), &["row-gap"]);
    let captured = vec![
        ".row { gap: 8px; }".into(),
        "@container (min-width: 500px){.row { gap: 32px; }}".into(),
    ];

    let styles = restored(&node, &captured);

    assert_eq!(styles["row-gap"], "8px");
}

/// No unconditional arm exists, so below the threshold the element takes its initial value,
/// which the recreation re-produces by saying nothing. Reached through a shorthand, this is
/// the one case that may still delete.
#[test]
fn drops_a_longhand_no_unconditional_shorthand_declared() {
    let node = decided(
        node("card", &[("background-color", "rgb(0, 255, 0)")]),
        &["background-color"],
    );
    let captured =
        vec!["@container (min-width: 500px){.card { background: rgb(0, 255, 0); }}".into()];

    let styles = restored(&node, &captured);

    assert!(!styles.contains_key("background-color"));
}
