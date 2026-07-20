use super::interaction_labels::{matches_trigger, semantic_trigger};
use crate::model::{Interaction, Node, PageState, Specification};
use std::collections::BTreeMap;

pub const FOCUS_CSS: &str = "[data-recreate-control]:focus-visible{outline:2px solid currentColor;outline-offset:2px}\
.recreateInteractionLayer{position:fixed;inset:0;z-index:2147480000;overflow:auto}\n";
pub const REDUCED_MOTION_CSS: &str = "@media(prefers-reduced-motion:reduce){\
*,*::before,*::after{animation:none!important;scroll-behavior:auto!important}}\n";

pub fn base_handlers(specification: &Specification, state: &PageState) -> BTreeMap<String, String> {
    let nodes = nodes_by_path(state);
    let mut handlers = BTreeMap::new();
    let shared_overflow = specification
        .interactions
        .iter()
        .filter(|interaction| {
            interaction
                .trigger_label
                .eq_ignore_ascii_case("More options")
        })
        .count()
        == 1;
    for (index, interaction) in specification.interactions.iter().enumerate() {
        let exact = nodes
            .get(interaction.trigger_path.as_str())
            .copied()
            .filter(|node| matches_trigger(interaction, node, state));
        let matches = if shared_overflow
            && interaction.trigger_occurrence.is_none()
            && interaction
                .trigger_label
                .eq_ignore_ascii_case("More options")
        {
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

pub fn state_handlers(
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
    let mut handlers = trigger
        .map(|node| {
            BTreeMap::from([(node.path.clone(), trigger_binding(Some(node), action, None))])
        })
        .unwrap_or_default();
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

pub fn closable(interaction: &Interaction, baselines: &[PageState]) -> bool {
    let label = interaction.trigger_label.to_ascii_lowercase();
    if (label == "more options" || label.contains("menu") || label.contains("launcher"))
        && interaction.states.iter().any(|state| {
            baselines
                .iter()
                .find(|baseline| baseline.viewport.width == state.viewport.width)
                .is_some_and(|baseline| {
                    if label == "more options" {
                        !crate::interaction_surface::roots(state, baseline).is_empty()
                    } else {
                        crate::interaction_state::surface_differs(
                            state,
                            baseline,
                            &interaction.trigger_path,
                            &interaction.trigger_label,
                        )
                    }
                })
        })
    {
        return true;
    }
    if baselines.iter().any(|baseline| {
        baseline
            .nodes
            .iter()
            .find(|node| node.path == interaction.trigger_path)
            .is_some_and(|node| {
                node.attributes.contains_key("aria-haspopup")
                    || node.attributes.contains_key("aria-expanded")
            })
    }) {
        return true;
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

fn closable_state(state: &PageState, baseline: &PageState) -> bool {
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

fn trigger_binding(node: Option<&Node>, action: &str, marker: Option<usize>) -> String {
    let native = node.is_some_and(native_control);
    let mut binding = format!("data-recreate-control=\"true\" onClick={{{action}}}");
    if let Some(marker) = marker {
        binding.push_str(&format!(" data-recreate-trigger=\"{marker}\""));
    }
    if !native {
        if !node.is_some_and(|node| node.attributes.contains_key("role")) {
            binding.push_str(" role=\"button\"");
        }
        if !node.is_some_and(|node| node.attributes.contains_key("tabindex")) {
            binding.push_str(" tabIndex={0}");
        }
        binding.push_str(&format!(
            " onKeyDown={{event=>keyActivate(event,{action})}}"
        ));
    }
    binding
}

fn native_control(node: &Node) -> bool {
    matches!(
        node.tag.as_str(),
        "button" | "summary" | "select" | "textarea"
    ) || node.tag == "input"
        || (node.tag == "a" && node.attributes.contains_key("href"))
}

fn focus_binding(node: &Node) -> String {
    let tab_index = if native_control(node) || node.attributes.contains_key("tabindex") {
        ""
    } else {
        " tabIndex={-1}"
    };
    format!("autoFocus ref={{element=>element?.focus({{preventScroll:true}})}}{tab_index}")
}

fn is_popup(node: &Node) -> bool {
    node.attributes
        .get("role")
        .is_some_and(|role| matches!(role.as_str(), "dialog" | "listbox" | "menu"))
        || node
            .attributes
            .get("aria-modal")
            .is_some_and(|value| value == "true")
}

fn nodes_by_path(state: &PageState) -> BTreeMap<&str, &Node> {
    state
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect()
}

fn append(handlers: &mut BTreeMap<String, String>, path: &str, value: &str) {
    handlers
        .entry(path.to_string())
        .and_modify(|binding| {
            binding.push_str(&format!(" {value}"));
        })
        .or_insert_with(|| value.to_string());
}

#[cfg(test)]
#[path = "interactions_tests.rs"]
mod tests;
