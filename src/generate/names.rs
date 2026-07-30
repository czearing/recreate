use crate::model::Node;
use std::collections::BTreeMap;

pub fn for_node(node: &Node, index: usize, nodes: &BTreeMap<String, &Node>) -> String {
    let value = node
        .attributes
        .get("data-testid")
        .or_else(|| node.attributes.get("aria-label"))
        .or_else(|| node.attributes.get("role"))
        .map(String::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| structural_name(node, nodes));
    let name = value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(capitalize)
        .collect::<String>();
    if name.is_empty() {
        format!("Component{}", index + 1)
    } else {
        name
    }
}

/// Names an unlabelled node from its own tag and immediate structure only.
/// Names inferred from captured text or aria-label vocabulary describe the one
/// page they were written against and are wrong on every other capture.
fn structural_name(node: &Node, nodes: &BTreeMap<String, &Node>) -> String {
    let has_child_tag = |tag: &str| {
        nodes.values().any(|candidate| {
            candidate.tag == tag && candidate.parent.as_deref() == Some(node.path.as_str())
        })
    };
    match node.tag.as_str() {
        "button" => "action-button".into(),
        "svg" => "icon".into(),
        "lineargradient" => "icon-linear-gradient".into(),
        "span" if has_child_tag("svg") => "icon-label".into(),
        "span" => "label".into(),
        "li" => "list-item".into(),
        "p" => "paragraph".into(),
        "div" => "content-section".into(),
        tag => tag.to_string(),
    }
}

fn capitalize(part: &str) -> String {
    let mut chars = part.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}
