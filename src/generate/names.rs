use super::component_identity;
use crate::model::Node;
use std::collections::BTreeMap;

/// Names a component from the strongest identity the page still carries.
///
/// The source application's own component name survives in scoped class names,
/// so that is preferred. A developer-authored test id is next, then the ARIA
/// role, then tag and structure. Accessible-name text is deliberately absent:
/// it is page copy, so it produced names like `CreateWeeklyPlanningAssistant`
/// that describe one sentence on one page and mean nothing on the next.
pub fn for_node(node: &Node, index: usize, nodes: &BTreeMap<String, &Node>) -> String {
    let value = node
        .attributes
        .get("class")
        .and_then(|class| component_identity::from_class(class))
        .or_else(|| node.attributes.get("data-testid").cloned())
        .or_else(|| node.attributes.get("role").cloned())
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
