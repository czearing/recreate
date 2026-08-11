//! The second emitter. An interaction state is compared against the baseline DOM by the
//! same one-sided difference the viewport bands used, and nothing masks the loss here:
//! no authored rule is copied for a state, so a silent delta renders the state
//! identically to the baseline it was meant to differ from.
use super::style_reversion_tests::box_node;
use crate::model::PageState;

const TARGET: &str = "html>body:nth-of-type(1)>div:nth-of-type(1)";

fn page(style: &[(&str, &str)]) -> PageState {
    let mut root = box_node("html", &[]);
    root.tag = "html".into();
    root.parent = None;
    let mut target = box_node(TARGET, style);
    target.parent = Some("html".into());
    PageState {
        nodes: vec![root, target],
        ..Default::default()
    }
}

fn surface(baseline: &PageState, state: &PageState) -> String {
    let alignment = crate::node_alignment::of(state, baseline);
    super::jsx_state_existing::existing_surface(
        state,
        &super::tree::Components {
            items: Vec::new(),
            by_root: Default::default(),
            children: Default::default(),
            classes: Default::default(),
            nodes: Default::default(),
        },
        &Default::default(),
        &Default::default(),
        &std::iter::once(TARGET.to_string()).collect(),
        &[],
        &alignment,
    )
}

#[test]
fn says_that_an_interaction_state_returned_a_property_to_its_default() {
    let markup = surface(&page(&[("background-color", "rgb(0, 128, 0)")]), &page(&[]));
    assert!(
        markup.contains("[\"background-color\",\"revert\"]"),
        "the state must undo the baseline declaration rather than send no delta: {markup}"
    );
}

/// Inverse guard: a state that changes nothing must still send nothing, so the fix
/// cannot buy the case above by emitting a delta for every element it visits.
#[test]
fn sends_no_state_delta_when_nothing_moved() {
    let style: &[(&str, &str)] = &[("background-color", "rgb(0, 128, 0)")];
    let markup = surface(&page(style), &page(style));
    assert!(markup.contains("styles={[]}"), "{markup}");
}

/// The same rule on the state axis: the baseline surface never declares a measurement,
/// so a state that stops reporting one has nothing to cancel.
#[test]
fn never_resets_a_measurement_the_baseline_rule_refused_to_declare() {
    let markup = surface(&page(&[("height", "32px")]), &page(&[]));
    assert!(
        !markup.contains("height"),
        "the baseline rule declares no sampled height, so no reset belongs here: {markup}"
    );
}
