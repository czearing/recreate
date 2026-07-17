use crate::model::{Interaction, Node, PageState};

pub fn semantic_trigger<'a>(interaction: &Interaction, state: &'a PageState) -> Option<&'a Node> {
    state
        .nodes
        .iter()
        .find(|node| matches_trigger(interaction, node, state))
}

pub fn matches_trigger(interaction: &Interaction, node: &Node, state: &PageState) -> bool {
    node.tag == interaction.trigger_tag && label(node, state) == interaction.trigger_label
}

fn label(node: &Node, state: &PageState) -> String {
    if let Some(label) = node
        .attributes
        .get("aria-label")
        .or_else(|| node.attributes.get("value"))
    {
        return normalize(label);
    }
    let prefix = format!("{}>", node.path);
    normalize(
        &state
            .nodes
            .iter()
            .filter(|candidate| candidate.tag == "#text" && candidate.path.starts_with(&prefix))
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
