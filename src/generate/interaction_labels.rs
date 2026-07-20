use crate::{
    behavior::{TriggerCandidate, TriggerKey, resolve_trigger},
    model::{Interaction, Node, PageState},
};

pub fn semantic_trigger<'a>(interaction: &Interaction, state: &'a PageState) -> Option<&'a Node> {
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
    let key = TriggerKey {
        path: interaction.trigger_path.clone(),
        tag: interaction.trigger_tag.clone(),
        label: interaction.trigger_label.clone(),
        occurrence: interaction.trigger_occurrence,
    };
    let path = resolve_trigger(&key, &candidates)?;
    state.nodes.iter().find(|node| node.path == path)
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
