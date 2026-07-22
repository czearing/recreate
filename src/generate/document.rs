use crate::model::{Node, PageState};
use std::collections::BTreeMap;

pub fn render(
    state: Option<&PageState>,
    mount: &str,
    classes: &BTreeMap<String, String>,
) -> String {
    let html = state
        .and_then(|state| state.nodes.iter().find(|node| node.tag == "html"))
        .map(attributes)
        .unwrap_or_default();
    let body = state
        .and_then(|state| state.nodes.iter().find(|node| node.tag == "body"))
        .map(attributes)
        .unwrap_or_default();
    let head_attributes = state
        .and_then(|state| state.nodes.iter().find(|node| node.tag == "head"))
        .map(|node| generated_class(node, classes))
        .unwrap_or_default();
    let head = state.map(|state| head(state, classes)).unwrap_or_else(|| {
        "<meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <link rel=\"icon\" href=\"data:,\"><title>Recreate</title>"
            .into()
    });
    format!(
        "<!doctype html><html{html}><head{head_attributes}>{head}</head><body{body}>{mount}\
         <script data-recreate-entry type=\"module\" src=\"/src/main.jsx\"></script></body></html>"
    )
}

fn head(state: &PageState, classes: &BTreeMap<String, String>) -> String {
    let Some(head) = state.nodes.iter().find(|node| node.tag == "head") else {
        return format!("<title>{}</title>", escape(&state.title));
    };
    state
        .nodes
        .iter()
        .filter(|node| node.parent.as_deref() == Some(head.path.as_str()))
        .filter(|node| {
            matches!(
                node.tag.as_str(),
                "base" | "link" | "meta" | "style" | "title"
            )
        })
        .map(|node| element(node, state, classes))
        .collect()
}

fn element(node: &Node, state: &PageState, classes: &BTreeMap<String, String>) -> String {
    let mut attributes = node
        .attributes
        .iter()
        .filter(|(name, _)| name.as_str() != "class")
        .map(|(name, value)| {
            if node.tag == "base" && name == "href" {
                return format!(" data-recreate-base-href=\"{}\"", escape(value));
            }
            format!(" {name}=\"{}\"", escape(value))
        })
        .collect::<String>();
    attributes.push_str(&generated_class(node, classes));
    if matches!(node.tag.as_str(), "base" | "link" | "meta") {
        return format!("<{}{attributes}>", node.tag);
    }

    let text = state
        .nodes
        .iter()
        .filter(|child| child.parent.as_deref() == Some(node.path.as_str()) && child.tag == "#text")
        .map(|child| child.text.as_str())
        .collect::<String>();
    let text = if node.tag == "style" {
        text.replace("</style", "<\\/style")
    } else {
        escape(&text)
    };
    format!("<{}{attributes}>{text}</{}>", node.tag, node.tag)
}

fn generated_class(node: &Node, classes: &BTreeMap<String, String>) -> String {
    classes
        .get(&node.path)
        .map(|class| format!(" class=\"{}\"", escape(class)))
        .unwrap_or_default()
}

fn attributes(node: &Node) -> String {
    node.attributes
        .iter()
        .filter(|(name, _)| !matches!(name.as_str(), "class" | "style"))
        .map(|(name, value)| format!(" {name}=\"{}\"", escape(value)))
        .collect()
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
