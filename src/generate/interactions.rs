use crate::model::{Interaction, Node, PageState, Specification};
use std::collections::BTreeMap;

pub const FOCUS_CSS: &str =
    "[data-recreate-control]:focus-visible{outline:2px solid currentColor;outline-offset:2px}\n";

pub fn base_handlers(specification: &Specification) -> BTreeMap<String, String> {
    let nodes = specification
        .states
        .first()
        .map(nodes_by_path)
        .unwrap_or_default();
    specification
        .interactions
        .iter()
        .enumerate()
        .map(|(index, interaction)| {
            let node = nodes.get(interaction.trigger_path.as_str()).copied();
            (
                interaction.trigger_path.clone(),
                trigger_binding(
                    node,
                    &format!("event=>activate(event,{})", index + 1),
                    Some(index + 1),
                ),
            )
        })
        .collect()
}

pub fn state_handlers(interaction: &Interaction, state: &PageState) -> BTreeMap<String, String> {
    let nodes = nodes_by_path(state);
    let trigger = nodes.get(interaction.trigger_path.as_str()).copied();
    let mut handlers = BTreeMap::from([(
        interaction.trigger_path.clone(),
        trigger_binding(trigger, "event=>onReset(event)", None),
    )]);
    let popup = state.nodes.iter().find(|node| is_popup(node));
    let focused = interaction
        .focused_path
        .as_deref()
        .and_then(|path| nodes.get(path).copied())
        .or(popup);
    if let Some(node) = focused {
        append(&mut handlers, &node.path, &focus_binding(node));
    }
    if let Some(popup) = popup {
        append(
            &mut handlers,
            &popup.path,
            "onKeyDown={event=>{if(event.key==='Escape'){event.stopPropagation();onReset()}}}",
        );
    }
    handlers
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
            binding.push(' ');
            binding.push_str(value);
        })
        .or_insert_with(|| value.to_string());
}

#[cfg(test)]
#[path = "interactions_tests.rs"]
mod tests;
