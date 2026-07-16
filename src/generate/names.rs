use crate::model::Node;

pub fn for_node(node: &Node, index: usize) -> String {
    let value = node
        .attributes
        .get("data-testid")
        .or_else(|| node.attributes.get("aria-label"))
        .or_else(|| node.attributes.get("role"))
        .map(String::as_str)
        .unwrap_or(match node.tag.as_str() {
            "button" => "action-button",
            "svg" => "icon",
            "span" => "label",
            "li" => "list-item",
            _ => "group",
        });
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

fn capitalize(part: &str) -> String {
    let mut chars = part.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Attributes, Rect, Styles};

    #[test]
    fn creates_semantic_component_names() {
        let mut node = Node {
            path: "a".into(),
            parent: None,
            tag: "div".into(),
            text: String::new(),
            attributes: Attributes::new(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            style: Styles::new(),
            before: None,
            after: None,
        };
        node.attributes
            .insert("data-testid".into(), "result-card".into());
        assert_eq!(for_node(&node, 0), "ResultCard");
    }
}
