//! Where the stage abstains.
//!
//! Each case is a way the evidence fails to identify a condition the recreation re-emits, and
//! in every one the measured value must survive untouched. Without these the repair would
//! withdraw values that nothing puts back.

use super::{decided, emitted, node, restored, scene};
/// A condition that was false at capture moved nothing, so the engine names no property and
/// the measured value is left alone. Without this the stage would withdraw a value nothing
/// puts back.
///
/// The second property carries the proof: `color` has no unconditional arm, so a stage that
/// skipped the evidence test would delete it.
#[test]
fn keeps_the_measured_value_when_the_condition_was_false_at_capture() {
    let node = node(
        "card",
        &[
            ("background-color", "rgb(255, 0, 0)"),
            ("color", "rgb(17, 17, 17)"),
        ],
    );
    let captured = vec![
        ".card { background-color: rgb(255, 0, 0); }".into(),
        "@media (max-width: 500px){.card { background-color: rgb(0, 0, 255); color: rgb(255, 255, 255); }}"
            .into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
    assert_eq!(styles["color"], "rgb(17, 17, 17)");
}

/// A rule is found through each class its subject compound names, so a node carrying one of
/// two is a candidate for a rule that needs both. The declaration this stage can see belongs
/// to an element the author never styled — but the engine still reports the property as
/// condition-decided, because some rule under that chain did decide it. Both arms are known
/// without knowing which rule: the base is the unconditional cascade's, and the override is
/// the value the capture measured with the chain in force. So the override is re-published on
/// this node's own generated class rather than left baked at every width.
#[test]
fn publishes_both_arms_where_the_node_only_partly_satisfies_the_compound() {
    let node = decided(node("card", &[("color", "rgb(0, 0, 255)")]), &["color"]);
    let captured = vec![
        ".card { color: rgb(255, 0, 0); }".into(),
        "@media (min-width: 600px){.card.featured { color: rgb(0, 0, 255); }}".into(),
    ];

    assert_eq!(restored(&node, &captured)["color"], "rgb(255, 0, 0)");
    assert!(
        emitted(&node, &captured)
            .contains(&"@media (min-width: 600px){.card{color:rgb(0, 0, 255);}}".to_string()),
        "{:?}",
        emitted(&node, &captured)
    );
}

/// A length is where abstaining is most tempting and most wrong: the index normally treats a
/// pixel literal disagreeing with the sample as a loser and returns the sample, which would
/// hand back the override this stage just identified.
#[test]
fn restores_an_authored_length_the_override_disagrees_with() {
    let node = decided(node("box", &[("padding-left", "40px")]), &["padding-left"]);
    let captured = vec![
        ".box { padding-left: 8px; }".into(),
        "@media (min-width: 900px){.box { padding-left: 40px; }}".into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["padding-left"], "8px");
}

/// The last condition still holding is the one that produced the sample, and the arm it is
/// withdrawn in favour of is the unconditional one — not the intermediate condition's.
#[test]
fn withdraws_to_the_unconditional_arm_through_two_stacked_conditions() {
    let node = decided(node("dial", &[("color", "rgb(0, 128, 128)")]), &["color"]);
    let captured = vec![
        ".dial { color: rgb(255, 0, 0); }".into(),
        "@media (min-width: 600px){.dial { color: rgb(0, 0, 255); }}".into(),
        "@media (min-width: 900px){.dial { color: rgb(0, 128, 128); }}".into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["color"], "rgb(255, 0, 0)");
}

/// A container query's condition is answered by an ancestor's box, which no viewport sweep
/// can enumerate, so it is the case that most needs the arm and it must not be viewport-shaped.
#[test]
fn repairs_a_container_query_by_the_same_rule() {
    let node = decided(
        node("card", &[("background-color", "rgb(59, 91, 219)")]),
        &["background-color"],
    );
    let captured = vec![
        ".card { background-color: rgb(233, 236, 239); }".into(),
        "@container cardwrap (min-width: 500px){.card { background-color: rgb(59, 91, 219); }}"
            .into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["background-color"], "rgb(233, 236, 239)");
}

/// `@supports` has one answer for the whole run and is not re-emitted, so withdrawing its
/// branch would delete a value nothing puts back. Only the document-answered rules qualify.
#[test]
fn leaves_a_condition_the_recreation_does_not_re_emit_alone() {
    let node = node("grid", &[("display", "grid")]);
    let captured = vec![
        ".grid { display: block; }".into(),
        "@supports (display: grid){.grid { display: grid; }}".into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["display"], "grid");
}

/// A rule inside a condition reaches the node through a relationship rather than by naming
/// it, so the matcher that rewrites selectors finds no override to rewrite. The engine still
/// reports the property as condition-decided, and both arms are measured, so the override is
/// re-published on this node's own generated class. Leaving it baked instead is what made a
/// component permanently unresponsive to the ancestor the author made it depend on.
#[test]
fn publishes_both_arms_of_a_conditional_rule_that_reaches_the_node_through_an_ancestor() {
    let node = decided(node("title", &[("color", "rgb(0, 0, 255)")]), &["color"]);
    let captured = vec![
        ".title { color: rgb(255, 0, 0); }".into(),
        "@media (min-width: 600px){.wrap .title { color: rgb(0, 0, 255); }}".into(),
    ];

    assert_eq!(restored(&node, &captured)["color"], "rgb(255, 0, 0)");
    assert!(
        emitted(&node, &captured)
            .contains(&"@media (min-width: 600px){.card{color:rgb(0, 0, 255);}}".to_string()),
        "{:?}",
        emitted(&node, &captured)
    );
}

/// A state rule inside a condition describes a state, not the resting value, and the state
/// emitter owns it. Withdrawing the resting arm on its evidence would misreport the element.
#[test]
fn leaves_a_state_rule_inside_a_condition_to_the_state_emitter() {
    let node = node("btn", &[("color", "rgb(0, 0, 255)")]);
    let captured = vec![
        ".btn { color: rgb(255, 0, 0); }".into(),
        "@media (min-width: 600px){.btn:hover { color: rgb(0, 0, 255); }}".into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["color"], "rgb(0, 0, 255)");
}

/// A prelude whose name merely begins with one of the two must not be swept in, or a future
/// at-rule the recreation does not re-emit silently loses its declarations.
#[test]
fn does_not_read_an_at_rule_whose_name_merely_begins_with_media() {
    let node = node("card", &[("color", "rgb(0, 0, 255)")]);
    let captured = vec![
        ".card { color: rgb(255, 0, 0); }".into(),
        "@media-hypothetical screen{.card { color: rgb(0, 0, 255); }}".into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["color"], "rgb(0, 0, 255)");
}

/// A layer is cascade position, not a condition, so a condition authored inside one is read
/// through the wrapper exactly as the re-emitter reads it. Otherwise the two disagree about
/// the same rule: one publishes the condition and the other keeps its branch in the base.
#[test]
fn reads_a_condition_authored_inside_a_layer() {
    let node = decided(node("card", &[("color", "rgb(0, 0, 255)")]), &["color"]);
    let captured = vec![
        ".card { color: rgb(255, 0, 0); }".into(),
        "@layer theme{@media (min-width: 600px){.card { color: rgb(0, 0, 255); }}}".into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["color"], "rgb(255, 0, 0)");
}

/// An axis shorthand resolves to one value per edge, so the branch a condition put the node
/// on is only ever a *part* of the value the author wrote. The value the withdrawal reads to
/// recognise that branch must therefore be read the same way the emission resolves it, or a
/// shorthand override is left baked into the base rule while every longhand one is repaired.
#[test]
fn withdraws_an_edge_a_conditional_axis_shorthand_put_the_node_on() {
    let node = decided(
        node("card", &[("margin-top", "5%"), ("margin-bottom", "15%")]),
        &["margin-top", "margin-bottom"],
    );
    let captured = vec![
        ".card { margin-top: 1px; margin-bottom: 2px; }".into(),
        "@media (min-width: 600px){.card { margin-block: 5% 15%; }}".into(),
    ];
    let styles = restored(&node, &captured);

    assert_eq!(styles["margin-top"], "1px");
    assert_eq!(styles["margin-bottom"], "2px");
}
#[path = "authored_conditions_base_arm_band_tests.rs"]
mod bands;

#[path = "authored_conditions_base_arm_evidence_tests.rs"]
mod evidence;
