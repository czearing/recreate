use crate::{
    behavior::{TriggerCandidate, TriggerKey, resolve_trigger},
    model::{Interaction, Node, PageState},
};

pub fn semantic_trigger<'a>(interaction: &Interaction, state: &'a PageState) -> Option<&'a Node> {
    semantic_key(
        &TriggerKey {
            path: interaction.trigger_path.clone(),
            tag: interaction.trigger_tag.clone(),
            label: interaction.trigger_label.clone(),
            occurrence: interaction.trigger_occurrence,
        },
        state,
    )
}

pub fn semantic_key<'a>(key: &TriggerKey, state: &'a PageState) -> Option<&'a Node> {
    let labeled = state
        .nodes
        .iter()
        .map(|node| (node, label(node, state)))
        .collect::<Vec<_>>();
    let candidates = labeled
        .iter()
        .map(|(node, label)| TriggerCandidate {
            path: &node.path,
            tag: &node.tag,
            label,
        })
        .collect::<Vec<_>>();
    let path = resolve_trigger(key, &candidates)?;
    state.nodes.iter().find(|node| node.path == path)
}

pub fn matches_trigger(interaction: &Interaction, node: &Node, state: &PageState) -> bool {
    matches_key(
        &TriggerKey {
            path: interaction.trigger_path.clone(),
            tag: interaction.trigger_tag.clone(),
            label: interaction.trigger_label.clone(),
            occurrence: interaction.trigger_occurrence,
        },
        node,
        state,
    )
}

pub fn matches_key(key: &TriggerKey, node: &Node, state: &PageState) -> bool {
    node.tag == key.tag && label(node, state) == key.label
}

fn label(node: &Node, state: &PageState) -> String {
    if let Some(label) = node.attributes.get("aria-label") {
        return normalize(label);
    }
    if let Some(placeholder) = node.attributes.get("placeholder") {
        return normalize(placeholder);
    }
    let prefix = format!("{}>", node.path);
    let text = normalize(
        &state
            .nodes
            .iter()
            .filter(|candidate| candidate.tag == "#text" && candidate.path.starts_with(&prefix))
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    );
    if !text.is_empty() {
        return text;
    }
    normalize(
        node.attributes
            .get("value")
            .map(String::as_str)
            .unwrap_or_default(),
    )
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
