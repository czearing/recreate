//! What counts as evidence that a condition produced the measured value.
//!
//! The stage recognises the branch by matching a conditional declaration against the sample.
//! These are the two ways that match can be satisfied without the condition having held, and
//! in both the measured value must survive.

use super::{node, restored};

/// The match must be against the sample **for that property**, not against the set of values
/// the node measured. A page reusing one colour for text and background — the common case,
/// not a contrived one — lets a conditional `color` coincide with the measured
/// `background-color` while the node's own `color` says the condition was false.
///
/// Without the per-property test the stage reads that coincidence as proof and withdraws
/// `color` to an arm that was never in force, painting the wrong colour at every width.
#[test]
fn does_not_read_another_property_measuring_the_same_value_as_proof() {
    let node = node(
        "card",
        &[
            ("background-color", "rgb(0, 0, 255)"),
            ("color", "rgb(255, 0, 0)"),
        ],
    );
    let captured = vec![
        ".card { color: rgb(17, 17, 17); }".into(),
        "@media (min-width: 900px){.card { color: rgb(0, 0, 255); }}".into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["color"], "rgb(255, 0, 0)");
}

/// The anti-vacuity twin. Identical shape, except the node really was captured on the
/// condition's branch, so the same code path must withdraw. Without this the test above
/// would also pass if the stage simply stopped withdrawing anything.
#[test]
fn still_withdraws_when_the_sample_for_that_property_does_match() {
    let node = node(
        "card",
        &[
            ("background-color", "rgb(0, 0, 255)"),
            ("color", "rgb(0, 0, 255)"),
        ],
    );
    let captured = vec![
        ".card { color: rgb(17, 17, 17); }".into(),
        "@media (min-width: 900px){.card { color: rgb(0, 0, 255); }}".into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["color"], "rgb(17, 17, 17)");
}

/// A CSS-wide keyword declares no value, so it is not an arm the base rule can publish.
/// Emitting the keyword itself would be worse than dropping: `revert` names the user-agent
/// origin, which the recreation's own cascade does not reach the same way, so the recreation
/// would paint something the source never did. Dropping lets the recreation's cascade
/// re-resolve the property exactly as the source's did.
#[test]
fn drops_rather_than_publishes_a_cascade_keyword_as_the_unconditional_arm() {
    let node = node("card", &[("color", "rgb(0, 0, 255)")]);
    let captured = vec![
        ".card { color: revert; }".into(),
        "@media (min-width: 900px){.card { color: rgb(0, 0, 255); }}".into(),
    ];
    let styles = restored(&node, &captured);

    assert!(
        !styles.contains_key("color"),
        "a keyword that declares no value must not become the published arm, got {:?}",
        styles.get("color")
    );
}
