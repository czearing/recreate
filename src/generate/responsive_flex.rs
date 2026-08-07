use crate::model::Node;
use std::collections::{HashMap, HashSet};

pub(in crate::generate) fn constrained_by_flex_chain(
    node: &Node,
    roots: &HashSet<&str>,
    nodes: &HashMap<&str, &Node>,
) -> bool {
    let mut parent = node.parent.as_deref();
    while let Some(path) = parent {
        if roots.contains(path) {
            return true;
        }
        let Some(node) = nodes.get(path) else {
            return false;
        };
        if node.style.get("display").map(String::as_str) != Some("flex")
            || node.style.get("flex-direction").map(String::as_str) != Some("row")
        {
            return false;
        }
        parent = node.parent.as_deref();
    }
    false
}

pub(super) fn fluid_flex_item(node: &Node, parent: Option<&Node>) -> bool {
    parent.is_some_and(|parent| {
        let flexible_main_axis = ["flex-grow", "flex-shrink"].into_iter().any(|name| {
            node.style
                .get(name)
                .and_then(|value| value.parse::<f64>().ok())
                .is_some_and(|value| value > 0.0)
        });
        parent.style.get("display").map(String::as_str) == Some("flex")
            && (parent
                .style
                .get("flex-direction")
                .map(String::as_str)
                .is_none_or(|direction| direction.starts_with("row"))
                && flexible_main_axis
                || (parent
                    .style
                    .get("align-items")
                    .map(String::as_str)
                    .is_none_or(|alignment| matches!(alignment, "normal" | "stretch"))
                    && node
                        .style
                        .get("align-self")
                        .map(String::as_str)
                        .is_none_or(|alignment| {
                            matches!(alignment, "auto" | "normal" | "stretch")
                        })))
            && !matches!(
                node.style.get("position").map(String::as_str),
                Some("absolute" | "fixed")
            )
            && node.attributes.get("role").is_none_or(|role| role != "img")
            && !(node.rect.width <= 32.0 && node.rect.height <= 32.0)
            && !matches!(
                node.tag.as_str(),
                "button" | "canvas" | "img" | "input" | "select" | "svg" | "textarea" | "video"
            )
    })
}

pub(in crate::generate) fn shrunk_flex_item(
    base: &Node,
    node: &Node,
    parent: Option<&Node>,
) -> bool {
    parent.is_some_and(|parent| {
        parent.style.get("display").map(String::as_str) == Some("flex")
            && parent.style.get("flex-direction").map(String::as_str) == Some("row")
            && node.rect.width <= parent.rect.width + 1.0
            && node
                .style
                .get("flex-shrink")
                .and_then(|value| value.parse::<f64>().ok())
                .is_some_and(|value| value > 0.0)
            && node.rect.width + 1.0 < base.rect.width
    })
}
