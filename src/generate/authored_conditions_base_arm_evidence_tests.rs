//! What counts as evidence that a condition produced the measured value.
//!
//! The engine answers it, per element and per property, and this stage may act only where its
//! own reach agrees. These are the ways that pair can be satisfied by one side alone, and in
//! every one of them the measured value must survive.

use super::{decided, node, restored};

/// The engine's answer is per property, so a page reusing one colour for text and background
/// — the common case, not a contrived one — cannot let a moved `background-color` stand in as
/// proof for a `color` no condition decided.
///
/// Without the per-property answer the stage reads that coincidence as proof and withdraws
/// `color` to an arm that was never in force, painting the wrong colour at every width.
#[test]
fn does_not_read_another_property_the_condition_moved_as_proof() {
    let node = decided(
        node(
            "card",
            &[
                ("background-color", "rgb(0, 0, 255)"),
                ("color", "rgb(255, 0, 0)"),
            ],
        ),
        &["background-color"],
    );
    let captured = vec![
        ".card { color: rgb(17, 17, 17); }".into(),
        "@media (min-width: 900px){.card { color: rgb(0, 0, 255); }}".into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["color"], "rgb(255, 0, 0)");
}

/// The anti-vacuity twin. Identical shape, except the engine names the property the rule
/// declares, so the same code path must withdraw. Without this the test above would also pass
/// if the stage simply stopped withdrawing anything.
#[test]
fn still_withdraws_when_the_engine_names_that_property() {
    let node = decided(
        node(
            "card",
            &[
                ("background-color", "rgb(0, 0, 255)"),
                ("color", "rgb(0, 0, 255)"),
            ],
        ),
        &["color"],
    );
    let captured = vec![
        ".card { color: rgb(17, 17, 17); }".into(),
        "@media (min-width: 900px){.card { color: rgb(0, 0, 255); }}".into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["color"], "rgb(17, 17, 17)");
}

/// The override is spelled in a vocabulary no computed sample uses, which is the whole
/// population the previous proof could not see: `0.5em` is never the string `8px`, and no
/// resolution of it belongs to this stage. The engine already decided, and said so.
#[test]
fn withdraws_an_override_whose_spelling_no_sample_could_ever_equal() {
    for override_value in [
        "0.5em",
        "5%",
        "calc(3px + 5px)",
        "min(8px, 3em)",
        "clamp(4px, 1em, 8px)",
        "10cqw",
        "1lh",
    ] {
        let node = decided(node("rel", &[("padding-left", "8px")]), &["padding-left"]);
        let captured = vec![
            ".rel { padding-left: 42px; }".to_string(),
            format!("@container (max-width: 400px){{.rel {{ padding-left: {override_value}; }}}}"),
        ];
        let styles = restored(&node, &captured);

        assert_eq!(
            styles["padding-left"], "42px",
            "an override spelled {override_value} kept no base arm"
        );
    }
}

/// A CSS-wide keyword declares no value, so it is not an arm the base rule can publish.
/// Emitting the keyword itself would be worse than dropping: `revert` names the user-agent
/// origin, which the recreation's own cascade does not reach the same way, so the recreation
/// would paint something the source never did. Dropping lets the recreation's cascade
/// re-resolve the property exactly as the source's did.
#[test]
fn drops_rather_than_publishes_a_cascade_keyword_as_the_unconditional_arm() {
    let node = decided(node("card", &[("color", "rgb(0, 0, 255)")]), &["color"]);
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
