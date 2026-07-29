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
        .unwrap_or_else(|| inferred_name(node, nodes));
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

fn inferred_name(node: &Node, nodes: &BTreeMap<String, &Node>) -> String {
    let direct = nodes
        .values()
        .copied()
        .filter(|candidate| candidate.parent.as_deref() == Some(node.path.as_str()))
        .collect::<Vec<_>>();
    let descendants = nodes
        .values()
        .copied()
        .filter(|candidate| {
            candidate
                .path
                .strip_prefix(&node.path)
                .is_some_and(|suffix| suffix.starts_with('>'))
        })
        .collect::<Vec<_>>();
    let texts = descendants
        .iter()
        .filter(|candidate| candidate.tag == "#text")
        .map(|candidate| candidate.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>();
    if descendants
        .iter()
        .any(|candidate| candidate.attributes.contains_key("data-title-text"))
    {
        return "notebook-title".into();
    }
    if descendants.iter().any(|candidate| {
        candidate
            .attributes
            .get("aria-label")
            .is_some_and(|label| label.to_ascii_lowercase().contains("more option"))
    }) {
        return "notebook-card-actions".into();
    }
    if texts.iter().any(|text| text == &"items") {
        return "notebook-item-count".into();
    }
    if texts.first().is_some_and(|text| *text == "+") {
        return "additional-collaborator-count".into();
    }
    if texts.len() == 1 && time_text(texts[0]) {
        return "notebook-updated-time".into();
    }
    if descendants
        .iter()
        .any(|candidate| candidate.tag == "button")
        && texts.contains(&"Create")
    {
        return if direct.iter().any(|candidate| candidate.tag == "button") {
            "suggested-card-content".into()
        } else {
            "suggested-card".into()
        };
    }
    if node.tag == "div" {
        let paragraphs = direct
            .iter()
            .filter(|candidate| candidate.tag == "p")
            .count();
        if paragraphs >= 2
            || paragraphs == 1
                && direct.iter().any(|child| {
                    child.tag == "div"
                        && descendants.iter().any(|candidate| {
                            candidate.tag == "p"
                                && candidate
                                    .path
                                    .strip_prefix(&child.path)
                                    .is_some_and(|suffix| suffix.starts_with('>'))
                        })
                })
        {
            return "suggested-card-copy".into();
        }
        if paragraphs == 1 {
            return "suggested-card-title-row".into();
        }
    }
    match node.tag.as_str() {
        "button" => "action-button".into(),
        "svg" => icon_name(node, nodes),
        "lineargradient" => "icon-linear-gradient".into(),
        "span" if node.attributes.contains_key("data-title-text") => "notebook-title-text".into(),
        "span"
            if node
                .parent
                .as_deref()
                .and_then(|path| nodes.get(path))
                .is_some_and(|parent| {
                    parent
                        .attributes
                        .get("aria-label")
                        .is_some_and(|label| label.starts_with("Change icon"))
                }) =>
        {
            "notebook-icon-glyph".into()
        }
        "span" if descendants.iter().any(|candidate| candidate.tag == "svg") => {
            "suggested-card-icon".into()
        }
        "span" => "label".into(),
        "li" => "list-item".into(),
        "p" if texts.iter().any(|text| text.len() > 64) => "suggested-card-description".into(),
        "p" => "suggested-card-title".into(),
        "div" => "content-section".into(),
        tag => tag.to_string(),
    }
}

fn icon_name(node: &Node, nodes: &BTreeMap<String, &Node>) -> String {
    if node
        .attributes
        .get("height")
        .is_some_and(|value| value == "100%")
    {
        return "notebook-card-background".into();
    }
    let label = node
        .parent
        .as_deref()
        .and_then(|path| nodes.get(path))
        .and_then(|parent| parent.attributes.get("aria-label"))
        .map(String::as_str)
        .unwrap_or_default();
    if label.contains("More options") {
        "more-options-icon".into()
    } else if label.contains("App launcher") {
        "app-launcher-icon".into()
    } else if label.contains("List view") || label.contains("Grid view") {
        "view-toggle-icon".into()
    } else if label.contains("Add content") || label.contains("Use voice") {
        "composer-action-icon".into()
    } else {
        "suggested-card-icon-graphic".into()
    }
}

fn time_text(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    ["ago", "yesterday", "today", "just now"]
        .iter()
        .any(|part| value.contains(part))
}

fn capitalize(part: &str) -> String {
    let mut chars = part.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}
