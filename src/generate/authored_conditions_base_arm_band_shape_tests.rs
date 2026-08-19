//! What a synthesized band may state, and what it must leave to the base rule.
//!
//! A band exists to restore what the unconditional rule gave up, so the two are decided
//! together: a property neither arm may state, one the node never baked, and one the band's
//! own shorthand already sets are all cases where restating it would be the defect.

use super::super::{credited, emitted, node, restored};
use super::{CARD, measured, token_scene};

/// A condition decides properties all over the page, most of them on elements and in slots no
/// generated class carries. Only a property this node bakes can be wrong in the output, and
/// only one it bakes can be put back on the class that bakes it, so the same intersection
/// bounds both directions — without it the recreation gains declarations the node never had.
#[test]
fn states_nothing_for_a_property_this_node_does_not_bake() {
    let node = measured(
        credited(
            node("arm-b", &[("padding-left", "62px")]),
            CARD,
            &["color", "padding-left"],
        ),
        &[("padding-left", "5px"), ("color", "rgb(255, 0, 0)")],
    );
    let mut captured = token_scene("arm-b", "--pad: 62px;");
    captured[0] = ".arm-b { padding-left: var(--pad); color: rgb(255, 0, 0); }".into();

    let styles = restored(&node, &captured);
    assert_eq!(styles["padding-left"], "5px");
    assert!(!styles.contains_key("color"), "{styles:?}");
    let rules = emitted(&node, &captured);
    assert!(
        !rules.iter().any(|rule| rule.contains("color:")),
        "{rules:?}"
    );
}

/// The band restates what the author wrote, which is often a shorthand, while the engine
/// answers in longhands. Adding the longhand beside the shorthand that already sets it states
/// one value twice and splits a selector list two elements shared, so what counts as "already
/// stated" is the same division the rest of this stage reads — the engine's own.
#[test]
fn adds_no_longhand_the_bands_own_shorthand_already_sets() {
    let node = measured(
        credited(
            node("card", &[("background-color", "rgb(0, 255, 0)")]),
            CARD,
            &["background-color"],
        ),
        &[("background-color", "rgb(255, 0, 0)")],
    );
    let captured = vec![
        ".card { background: rgb(255, 0, 0); }".to_string(),
        format!("{CARD}{{.card {{ background: rgb(0, 255, 0); }}}}"),
    ];

    assert_eq!(
        emitted(&node, &captured),
        vec![format!("{CARD}{{.card{{background: rgb(0, 255, 0);}}}}")]
    );
}

/// Withdrawing a padding moves the used width of a block that has none of its own. The engine
/// reports it, truthfully, as decided — but the author declared it nowhere, so neither arm may
/// state it: the recreation drops it and layout re-derives it from the padding that did move,
/// exactly as the source does. Stating the measurement in the band would pin one container's
/// box onto every instance of the component.
#[test]
fn states_no_arm_for_a_used_value_the_withdrawal_only_reflowed() {
    let node = measured(
        credited(
            node("arm-b", &[("padding-left", "62px"), ("width", "178px")]),
            CARD,
            &["padding-left", "width"],
        ),
        &[("padding-left", "5px"), ("width", "235px")],
    );
    let captured = token_scene("arm-b", "--pad: 62px;");

    let styles = restored(&node, &captured);
    assert_eq!(styles["padding-left"], "5px");
    assert!(!styles.contains_key("width"), "{styles:?}");
    let rules = emitted(&node, &captured);
    assert!(
        !rules.iter().any(|rule| rule.contains("width:178px")),
        "{rules:?}"
    );
}
