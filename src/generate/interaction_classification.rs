use super::bindings::is_popup;
use crate::generate::interaction_labels::matches_trigger;
use crate::model::{Interaction, PageState};

pub fn closable(interaction: &Interaction, baselines: &[PageState]) -> bool {
    if state_control(interaction, baselines) {
        return false;
    }
    let comparisons = interaction
        .states
        .iter()
        .filter_map(|state| {
            baselines
                .iter()
                .find(|baseline| baseline.viewport.width == state.viewport.width)
                .map(|baseline| (state, baseline))
        })
        .collect::<Vec<_>>();
    let closable = comparisons
        .iter()
        .map(|(state, baseline)| closable_state(state, baseline))
        .collect::<Vec<_>>();
    closable.iter().filter(|value| **value).count() * 2 > closable.len()
}

pub fn shared_trigger(interaction: &Interaction, baselines: &[PageState]) -> bool {
    interaction.trigger_occurrence.is_none()
        && baselines.iter().any(|baseline| {
            baseline
                .nodes
                .iter()
                .filter(|node| matches_trigger(interaction, node, baseline))
                .count()
                > 1
        })
}

pub fn state_control(interaction: &Interaction, baselines: &[PageState]) -> bool {
    baselines.iter().any(|baseline| {
        baseline
            .nodes
            .iter()
            .find(|node| node.path == interaction.trigger_path)
            .is_some_and(|node| {
                !node.attributes.contains_key("aria-haspopup")
                    && (node
                        .attributes
                        .get("role")
                        .is_some_and(|role| role == "tab")
                        || node.attributes.contains_key("aria-pressed")
                        || node.attributes.contains_key("aria-selected"))
            })
    })
}

pub(super) fn closable_state(state: &PageState, baseline: &PageState) -> bool {
    if state.nodes.iter().any(is_popup) {
        return true;
    }
    let baseline_paths = baseline
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let state_paths = state
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    baseline_paths.symmetric_difference(&state_paths).count() >= 8
}
