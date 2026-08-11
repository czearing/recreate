use super::interaction_scroll;
use super::scroll_state_tests::state;
use crate::model::{Interaction, PageState, Specification};

/// Two rails identical but for where they came to rest, moved to the same place by one action.
/// `#pinned` rests at 400, `#fresh` at 0, and the action puts both at 900.
fn twin_rails() -> Specification {
    specification(
        state(&[("pinned", 400.0, 0.0), ("fresh", 0.0, 0.0)]),
        state(&[("pinned", 900.0, 0.0), ("fresh", 900.0, 0.0)]),
    )
}

fn specification(baseline: PageState, after: PageState) -> Specification {
    Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![baseline],
        interactions: vec![Interaction {
            trigger_path: "html>body>button".into(),
            trigger_tag: "button".into(),
            trigger_label: "Advance".into(),
            trigger_occurrence: None,
            focused_path: None,
            states: vec![after],
        }],
        transitions: Vec::new(),
    }
}

/// The filed defect. The runtime replays the emitted triple through `element.scrollTo`, which
/// per CSSOM View sets an absolute position, so the number must be the offset the element
/// held. Two rails that end at the same place must be serialized with the same number
/// whatever offset they started from.
#[test]
fn serializes_the_same_position_for_rails_that_ended_together() {
    let targets = interaction_scroll::targets(&twin_rails());
    assert!(
        targets.contains("[\"pinned\", 900,0]"),
        "a rail resting at 400 and driven to 900 must serialize 900, got {targets}"
    );
    assert!(
        targets.contains("[\"fresh\", 900,0]"),
        "a rail resting at 0 and driven to 900 must serialize 900, got {targets}"
    );
}

/// An element whose resting offset is zero must serialize exactly what it always did, so the
/// correction is provably inert wherever the capture started from rest.
#[test]
fn leaves_an_element_that_started_at_rest_unchanged() {
    let specification = specification(state(&[("pane", 0.0, 0.0)]), state(&[("pane", 0.0, 640.0)]));
    assert!(interaction_scroll::targets(&specification).contains("[\"pane\", 0,640]"));
}

/// `scrollTo` clamps a negative target to zero, so a differential value replays correctly here
/// and the render cannot tell the two kinds apart. The emitted number must still be the
/// position the element held, which is zero, not the distance it travelled.
#[test]
fn serializes_a_return_to_the_top_as_the_position_zero() {
    let specification = specification(state(&[("pane", 0.0, 300.0)]), state(&[("pane", 0.0, 0.0)]));
    let targets = interaction_scroll::targets(&specification);
    assert!(
        targets.contains("[\"pane\", 0,0]"),
        "a pane returned to the top holds offset 0, got {targets}"
    );
}

/// An element already resting at a horizontal offset that only moves vertically has not moved
/// horizontally. Reading the emitted position would say it had, and the carousel runtime would
/// be armed for a page with no horizontal movement.
#[test]
fn does_not_call_a_resting_horizontal_offset_a_horizontal_move() {
    let baseline = state(&[("rail", 400.0, 0.0)]);
    let after = state(&[("rail", 400.0, 250.0)]);
    let specification = specification(baseline.clone(), after);
    assert!(!interaction_scroll::moves_horizontally(
        &specification.interactions[0],
        &specification.states
    ));
}

/// The differential question is still asked correctly when the element does move on the axis.
#[test]
fn still_reports_a_horizontal_move_from_a_displaced_rest() {
    let specification = specification(
        state(&[("rail", 400.0, 0.0)]),
        state(&[("rail", 900.0, 0.0)]),
    );
    assert!(interaction_scroll::moves_horizontally(
        &specification.interactions[0],
        &specification.states
    ));
}

/// Selection stays differential even though the value is not. An element that held its offset
/// through the action did not move, so it belongs to the initial-scroll table and must not be
/// re-stated in the interaction snapshot — where it would overwrite whatever the live merge
/// had on the other axis.
#[test]
fn omits_a_rail_that_held_its_offset_through_the_action() {
    let specification = specification(
        state(&[("held", 400.0, 0.0), ("mover", 0.0, 0.0)]),
        state(&[("held", 400.0, 0.0), ("mover", 900.0, 0.0)]),
    );
    let targets = interaction_scroll::targets(&specification);
    assert!(targets.contains("[\"mover\", 900,0]"));
    assert!(
        !targets.contains("\"held\""),
        "an unmoved rail must not enter the interaction snapshot, got {targets}"
    );
}

/// The document's scroll reaches the window slot, and it is a position there too.
#[test]
fn serializes_the_document_position_into_the_window_slot() {
    let mut baseline = state(&[]);
    let mut after = state(&[]);
    for (target, value) in [(&mut baseline, 200.0), (&mut after, 950.0)] {
        target.dom.insert(
            "html".into(),
            crate::model::DomNode {
                scroll_top: value,
                ..Default::default()
            },
        );
    }
    let targets = interaction_scroll::targets(&specification(baseline, after));
    assert!(
        targets.contains("window:[0,950]"),
        "the document holds offset 950, got {targets}"
    );
}
