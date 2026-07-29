use super::classification::state_control;
use crate::model::{Interaction, Node, PageState};
use std::collections::BTreeMap;

pub(super) fn trigger_binding(node: Option<&Node>, action: &str, marker: Option<usize>) -> String {
    if node.is_some_and(text_entry_control) {
        let marker = marker
            .map(|value| format!(" data-recreate-trigger=\"{value}\""))
            .unwrap_or_default();
        let action = action.strip_prefix("event=>").unwrap_or(action);
        let action = action.strip_suffix(')').map_or_else(
            || action.to_string(),
            |prefix| format!("{prefix},event.currentTarget.value.length>0)"),
        );
        return format!(
            "data-recreate-control=\"true\"{marker} \
             onInput={{event=>{{{action}}}}}"
        );
    }

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

pub fn transition_key(transition: &crate::model::InteractionTransition) -> String {
    serde_json::to_string(&(
        &transition.trigger_path,
        &transition.trigger_tag,
        &transition.trigger_label,
        transition.trigger_occurrence,
    ))
    .unwrap()
}

pub(super) fn transition_binding(node: &Node, key: &str) -> String {
    let key = serde_json::to_string(key).unwrap();
    let native = native_control(node);
    let mut binding = format!("data-recreate-control=\"true\" data-recreate-trigger={{{key}}}");
    if !native {
        if !node.attributes.contains_key("role") {
            binding.push_str(" role=\"button\"");
        }
        if !node.attributes.contains_key("tabindex") {
            binding.push_str(" tabIndex={0}");
        }
    }
    binding
}

pub fn rendered(interaction: &Interaction, baselines: &[PageState]) -> bool {
    !interaction.states.is_empty()
        || text_entry_interaction(interaction)
        || state_control(interaction, baselines)
}

pub fn text_entry_interaction(interaction: &Interaction) -> bool {
    interaction.trigger_tag == "textarea"
        || (interaction.trigger_tag == "input"
            && !interaction.trigger_label.eq_ignore_ascii_case("Search"))
}

pub(super) fn text_entry_control(node: &Node) -> bool {
    node.tag == "textarea"
        || (node.tag == "input"
            && node
                .attributes
                .get("type")
                .is_none_or(|value| value.eq_ignore_ascii_case("text")))
}

pub(super) fn native_control(node: &Node) -> bool {
    matches!(
        node.tag.as_str(),
        "button" | "summary" | "select" | "textarea"
    ) || node.tag == "input"
        || (node.tag == "a" && node.attributes.contains_key("href"))
}

pub(super) fn focus_binding(node: &Node) -> String {
    let tab_index = if native_control(node) || node.attributes.contains_key("tabindex") {
        ""
    } else {
        " tabIndex={-1}"
    };
    format!("autoFocus ref={{element=>element?.focus({{preventScroll:true}})}}{tab_index}")
}

pub(super) fn is_popup(node: &Node) -> bool {
    node.attributes
        .get("role")
        .is_some_and(|role| matches!(role.as_str(), "dialog" | "listbox" | "menu"))
        || node
            .attributes
            .get("aria-modal")
            .is_some_and(|value| value == "true")
}

pub(super) fn nodes_by_path(state: &PageState) -> BTreeMap<&str, &Node> {
    state
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect()
}

pub(super) fn append(handlers: &mut BTreeMap<String, String>, path: &str, value: &str) {
    handlers
        .entry(path.to_string())
        .and_modify(|binding| {
            binding.push_str(&format!(" {value}"));
        })
        .or_insert_with(|| value.to_string());
}
