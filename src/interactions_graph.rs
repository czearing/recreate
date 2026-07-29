use super::interactions_scripts::Candidate;
use crate::model::{Interaction, InteractionAction, InteractionTransition, PageState};

pub(super) fn edge(
    from_state: usize,
    to_state: usize,
    interaction: &Interaction,
) -> InteractionTransition {
    InteractionTransition {
        from_state,
        to_state,
        action: InteractionAction::Activate,
        trigger_path: interaction.trigger_path.clone(),
        trigger_tag: interaction.trigger_tag.clone(),
        trigger_label: interaction.trigger_label.clone(),
        trigger_occurrence: interaction.trigger_occurrence,
    }
}

pub(super) fn edge_candidate(
    from_state: usize,
    to_state: usize,
    candidate: &Candidate,
) -> InteractionTransition {
    edge_candidate_action(from_state, to_state, candidate, InteractionAction::Activate)
}

pub(super) fn edge_candidate_action(
    from_state: usize,
    to_state: usize,
    candidate: &Candidate,
    action: InteractionAction,
) -> InteractionTransition {
    InteractionTransition {
        from_state,
        to_state,
        action,
        trigger_path: candidate.path.clone(),
        trigger_tag: candidate.tag.clone(),
        trigger_label: candidate.label.clone(),
        trigger_occurrence: Some(candidate.occurrence),
    }
}

pub(super) fn same_edge(left: &InteractionTransition, right: &InteractionTransition) -> bool {
    left.from_state == right.from_state
        && left.to_state == right.to_state
        && left.action == right.action
        && left.trigger_path == right.trigger_path
        && left.trigger_tag == right.trigger_tag
        && left.trigger_label == right.trigger_label
        && left.trigger_occurrence == right.trigger_occurrence
}

pub(super) fn candidate(interaction: &Interaction, baseline: &PageState) -> Candidate {
    let state_control = baseline
        .nodes
        .iter()
        .find(|node| node.path == interaction.trigger_path)
        .is_some_and(|node| {
            node.attributes
                .get("role")
                .is_some_and(|role| role == "tab")
                || node.attributes.contains_key("aria-pressed")
                || node.attributes.contains_key("aria-selected")
        });
    Candidate {
        path: interaction.trigger_path.clone(),
        tag: interaction.trigger_tag.clone(),
        label: interaction.trigger_label.clone(),
        occurrence: interaction.trigger_occurrence.unwrap_or_default(),
        disabled: false,
        navigates: false,
        state_control,
    }
}

pub(super) fn state_matches(left: &PageState, right: &PageState) -> bool {
    left.nodes.len() == right.nodes.len()
        && left.nodes.iter().zip(&right.nodes).all(|(left, right)| {
            left.path == right.path
                && left.tag == right.tag
                && left.text == right.text
                && left.attributes == right.attributes
                && (left.rect.x - right.rect.x).abs() <= 1.0
                && (left.rect.y - right.rect.y).abs() <= 1.0
                && (left.rect.width - right.rect.width).abs() <= 1.0
                && (left.rect.height - right.rect.height).abs() <= 1.0
        })
}

pub(super) fn visual_state_matches(left: &PageState, right: &PageState) -> bool {
    state_matches(left, right)
        && left
            .nodes
            .iter()
            .zip(&right.nodes)
            .all(|(left, right)| left.style == right.style)
}
