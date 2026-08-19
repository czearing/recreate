use super::interactions_scripts::Candidate;
use crate::{
    interaction_state,
    model::{Interaction, PageState},
};
use std::collections::HashMap;

pub fn deduplicate(interactions: &mut Vec<Interaction>) {
    let mut positions = HashMap::<(String, String, String), usize>::new();
    let mut unique: Vec<Interaction> = Vec::new();
    for interaction in interactions.drain(..) {
        let key = (
            interaction.trigger_path.clone(),
            interaction.trigger_tag.clone(),
            interaction.trigger_label.clone(),
        );
        if let Some(index) = positions.get(&key).copied() {
            if evidence_score(&interaction) > evidence_score(&unique[index]) {
                unique[index] = interaction;
            }
        } else {
            positions.insert(key, unique.len());
            unique.push(interaction);
        }
    }
    *interactions = unique;
}

pub(super) fn evidence_score(interaction: &Interaction) -> usize {
    interaction
        .states
        .iter()
        .map(|state| state.nodes.len())
        .sum()
}

pub(super) fn responsive_baselines<'a>(
    text_entry: bool,
    state_control: bool,
    state: &PageState,
    baseline: &PageState,
    trigger: &str,
    label: &str,
    baselines: &'a [PageState],
) -> Vec<&'a PageState> {
    if state_control
        || text_entry
        || interaction_state::surface_differs(state, baseline, trigger, label)
        || geometry_differs(state, baseline)
    {
        baselines.iter().skip(1).collect()
    } else {
        Vec::new()
    }
}

#[cfg(test)]
pub(super) fn discovery_differs(
    label: &str,
    trigger: &str,
    left: &PageState,
    right: &PageState,
) -> bool {
    interaction_state::meaningfully_differs(left, right)
        || interaction_state::surface_differs(left, right, trigger, label)
        || geometry_differs(left, right)
}

pub(super) fn geometry_differs(left: &PageState, right: &PageState) -> bool {
    left.nodes
        .iter()
        .filter(|node| actionable_node(node))
        .any(|node| {
            right
                .nodes
                .iter()
                .find(|baseline| baseline.path == node.path)
                .is_some_and(|baseline| {
                    (node.rect.x - baseline.rect.x).abs() > 1.0
                        || (node.rect.y - baseline.rect.y).abs() > 1.0
                })
        })
}

pub(super) fn actionable_node(node: &crate::model::Node) -> bool {
    matches!(
        node.tag.as_str(),
        "a" | "button" | "input" | "select" | "textarea" | "summary"
    ) || node.attributes.contains_key("role")
        || node.attributes.contains_key("tabindex")
        || node.attributes.contains_key("aria-label")
        || node.attributes.contains_key("aria-haspopup")
}

pub(super) fn surface_backed(interaction: &Interaction, baselines: &[PageState]) -> bool {
    interaction.states.iter().any(|state| {
        baselines
            .iter()
            .find(|baseline| baseline.viewport.width == state.viewport.width)
            .is_some_and(|baseline| !crate::interaction_surface::roots(state, baseline).is_empty())
    })
}

pub(super) fn action_family(label: &str) -> Option<&str> {
    let (action, subject) = label.split_once(" for ")?;
    (!action.trim().is_empty() && !subject.trim().is_empty()).then_some(action.trim())
}

pub(super) fn candidate_relevant(
    candidate: &Candidate,
    baseline: &PageState,
    reached: &PageState,
    trigger: Option<&str>,
    popup: bool,
) -> bool {
    if trigger == Some(candidate.path.as_str()) {
        return true;
    }
    if candidate.state_control {
        return !popup;
    }
    let current = reached
        .nodes
        .iter()
        .find(|node| node.path == candidate.path);
    let original = baseline
        .nodes
        .iter()
        .find(|node| node.path == candidate.path);
    match (current, original) {
        (Some(_), None) => true,
        (Some(current), Some(original)) => {
            transition_attributes(current) != transition_attributes(original)
        }
        _ => false,
    }
}

pub(super) fn popup_state(state: &PageState, baseline: &PageState) -> bool {
    let baseline_paths = baseline
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .collect::<std::collections::HashSet<_>>();
    state.nodes.iter().any(|node| {
        !baseline_paths.contains(node.path.as_str())
            && (node
                .attributes
                .get("aria-modal")
                .is_some_and(|value| value == "true")
                || node.attributes.get("role").is_some_and(|role| {
                    matches!(
                        role.as_str(),
                        "dialog" | "menu" | "listbox" | "tooltip" | "alertdialog"
                    )
                }))
    })
}

pub(super) fn overlay_state(state: &PageState, baseline: &PageState) -> bool {
    if popup_state(state, baseline) {
        return true;
    }
    let baseline_paths = baseline
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .collect::<std::collections::HashSet<_>>();
    state.nodes.iter().any(|node| {
        !baseline_paths.contains(node.path.as_str())
            && super::interactions_evidence::actionable_node(node)
    })
}

fn transition_attributes(node: &crate::model::Node) -> Vec<(&str, &str)> {
    node.attributes
        .iter()
        .filter(|(name, _)| {
            name.starts_with("aria-")
                || matches!(
                    name.as_str(),
                    "disabled" | "hidden" | "checked" | "selected" | "value" | "tabindex"
                )
        })
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

pub(super) struct InteractionAlias {
    pub(super) candidate: Candidate,
    pub(super) template_path: String,
    pub(super) template_tag: String,
    pub(super) template_label: String,
}
