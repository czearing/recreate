use crate::model::Node;
use std::collections::HashMap;

pub fn child_nodes(nodes: &[Node]) -> HashMap<&str, Vec<&Node>> {
    let mut children = HashMap::new();
    for node in nodes {
        if let Some(parent) = node.parent.as_deref() {
            children.entry(parent).or_insert_with(Vec::new).push(node);
        }
    }
    children
}

pub fn multiline_text_box(node: &Node) -> bool {
    node.style
        .get("line-height")
        .and_then(|value| value.strip_suffix("px"))
        .and_then(|value| value.parse::<f64>().ok())
        .is_some_and(|line_height| node.rect.height > line_height * 1.5)
}

pub fn important_interaction_paint(css: &str) -> String {
    css.split_inclusive(';')
        .map(|declaration| {
            let property = declaration
                .split_once(':')
                .map(|(property, _)| property)
                .unwrap_or_default();
            if (matches!(
                property,
                "background-color"
                    | "border"
                    | "color"
                    | "fill"
                    | "stroke"
                    | "-webkit-text-fill-color"
            ) || property.starts_with("border-"))
                && !declaration.contains("!important")
            {
                format!("{}!important;", declaration.trim_end_matches(';'))
            } else {
                declaration.to_string()
            }
        })
        .collect()
}

pub fn flex_direction(node: &Node, children: &[&Node]) -> Option<&'static str> {
    if node
        .style
        .get("display")
        .is_none_or(|value| value != "flex")
    {
        return None;
    }
    let direction = node.style.get("flex-direction")?.as_str();
    let first = children
        .iter()
        .find(|child| child.rect.width > 0.0 && child.rect.height > 0.0)?;
    let last = children
        .iter()
        .rev()
        .find(|child| child.rect.width > 0.0 && child.rect.height > 0.0)?;
    match direction {
        "row" if first.rect.x > last.rect.x + 1.0 => Some("row-reverse"),
        "column" if first.rect.y > last.rect.y + 1.0 => Some("column-reverse"),
        _ => None,
    }
}

pub fn inferred_float(node: &Node, parent: Option<&Node>) -> Option<&'static str> {
    let parent = parent?;
    let missing_float = node.style.get("float").is_none_or(|value| value == "none");
    let right_edge = parent.rect.x + parent.rect.width;
    (missing_float
        && parent
            .style
            .get("display")
            .is_some_and(|value| value == "block")
        && node
            .style
            .get("display")
            .is_some_and(|value| value == "block")
        && node
            .style
            .get("position")
            .is_some_and(|value| value == "static")
        && node.rect.width <= 0.5
        && (node.rect.x - right_edge).abs() <= 1.0)
        .then_some("right")
}
