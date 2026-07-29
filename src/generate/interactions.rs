use super::interaction_labels::{matches_trigger, semantic_key, semantic_trigger};
use crate::{
    behavior::TriggerKey,
    model::{Interaction, PageState, Specification},
};
use std::collections::BTreeMap;

pub const FOCUS_CSS: &str = ".recreateInteractionLayer{position:fixed;inset:0;overflow:auto}\
.recreateAnchoredSurface{display:contents}\n";
pub const REDUCED_MOTION_CSS: &str = "@media(prefers-reduced-motion:reduce){\
*,*::before,*::after{animation:none!important;scroll-behavior:auto!important}}\n";

pub fn base_handlers(specification: &Specification, state: &PageState) -> BTreeMap<String, String> {
    if !specification.transitions.is_empty() {
        return transition_handlers(specification, state, 0);
    }
    let nodes = nodes_by_path(state);
    let mut handlers = BTreeMap::new();
    for (index, interaction) in specification.interactions.iter().enumerate() {
        let exact = nodes
            .get(interaction.trigger_path.as_str())
            .copied()
            .filter(|node| matches_trigger(interaction, node, state));
        let matches = if shared_trigger(interaction, std::slice::from_ref(state)) {
            state
                .nodes
                .iter()
                .filter(|node| matches_trigger(interaction, node, state))
                .collect::<Vec<_>>()
        } else {
            exact
                .or_else(|| semantic_trigger(interaction, state))
                .into_iter()
                .collect()
        };
        for node in matches {
            handlers.insert(
                node.path.clone(),
                trigger_binding(
                    Some(node),
                    &format!("event=>activate(event,{})", index + 1),
                    Some(index + 1),
                ),
            );
        }
    }
    handlers
}

pub fn transition_handlers(
    specification: &Specification,
    state: &PageState,
    _from_state: usize,
) -> BTreeMap<String, String> {
    let nodes = nodes_by_path(state);
    let mut handlers = BTreeMap::new();
    let mut bound = std::collections::BTreeSet::new();
    for transition in &specification.transitions {
        let key = TriggerKey {
            path: transition.trigger_path.clone(),
            tag: transition.trigger_tag.clone(),
            label: transition.trigger_label.clone(),
            occurrence: transition.trigger_occurrence,
        };
        let trigger = nodes
            .get(transition.trigger_path.as_str())
            .copied()
            .filter(|node| node.tag == transition.trigger_tag)
            .or_else(|| semantic_key(&key, state));
        if let Some(node) = trigger {
            let token = transition_key(transition);
            if !bound.insert((node.path.clone(), token.clone())) {
                continue;
            }
            append(&mut handlers, &node.path, &transition_binding(node, &token));
        }
    }
    handlers
}

pub fn state_handlers(
    specification: &Specification,
    state_id: usize,
    interaction: &Interaction,
    state: &PageState,
    baseline: &PageState,
) -> BTreeMap<String, String> {
    let nodes = nodes_by_path(state);
    let trigger = nodes
        .get(interaction.trigger_path.as_str())
        .copied()
        .filter(|node| matches_trigger(interaction, node, state))
        .or_else(|| semantic_trigger(interaction, state));
    let action = if closable_state(state, baseline) {
        "event=>onReset(event)"
    } else {
        "event=>event.preventDefault()"
    };
    let mut handlers = if specification.transitions.is_empty() {
        trigger
            .map(|node| {
                BTreeMap::from([(node.path.clone(), trigger_binding(Some(node), action, None))])
            })
            .unwrap_or_default()
    } else {
        transition_handlers(specification, state, state_id)
    };
    let popup = state.nodes.iter().find(|node| is_popup(node));
    let focused = interaction
        .focused_path
        .as_deref()
        .and_then(|path| nodes.get(path).copied())
        .or(popup);
    if let Some(node) = focused {
        append(&mut handlers, &node.path, &focus_binding(node));
    }

    let baseline_paths = baseline
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let roots: Vec<_> = state
        .nodes
        .iter()
        .filter(|node| {
            !baseline_paths.contains(node.path.as_str())
                && node
                    .parent
                    .as_deref()
                    .is_none_or(|parent| baseline_paths.contains(parent))
        })
        .collect();
    let viewport_area = f64::from(state.viewport.width) * f64::from(state.viewport.height);
    let compact = roots
        .iter()
        .any(|node| is_popup(node) || node.rect.width * node.rect.height < viewport_area * 0.8);
    for node in roots.into_iter().filter(|node| {
        !compact || is_popup(node) || node.rect.width * node.rect.height < viewport_area * 0.8
    }) {
        append(&mut handlers, &node.path, "data-recreate-surface=\"true\"");
    }
    for root in crate::interaction_surface::roots(state, baseline) {
        append(&mut handlers, &root, "data-recreate-surface=\"true\"");
    }
    handlers
}

#[path = "interaction_bindings.rs"]
mod bindings;
#[path = "interaction_classification.rs"]
mod classification;

use bindings::{
    append, focus_binding, is_popup, nodes_by_path, transition_binding, trigger_binding,
};
pub use bindings::{rendered, text_entry_interaction, transition_key};
use classification::closable_state;
pub use classification::{closable, shared_trigger, state_control};

#[cfg(test)]
#[path = "interactions_tests.rs"]
mod tests;
