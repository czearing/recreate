use crate::model::{Node, PageState};
use std::collections::BTreeMap;

pub fn render(
    state: Option<&PageState>,
    mount: &str,
    classes: &BTreeMap<String, String>,
    assets: &BTreeMap<String, String>,
) -> String {
    let html = state
        .and_then(|state| state.nodes.iter().find(|node| node.tag == "html"))
        .map(|node| attributes(node, classes, assets))
        .unwrap_or_default();
    let body = state
        .and_then(|state| state.nodes.iter().find(|node| node.tag == "body"))
        .map(|node| attributes(node, classes, assets))
        .unwrap_or_default();
    let head_attributes = state
        .and_then(|state| state.nodes.iter().find(|node| node.tag == "head"))
        .map(|node| attributes(node, classes, assets))
        .unwrap_or_default();
    let head = state
        .map(|state| head(state, classes, assets))
        .unwrap_or_else(|| {
        "<meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <link rel=\"icon\" href=\"data:,\"><title>Recreate</title>"
            .into()
    });
    format!(
        "<!doctype html><html{html}><head{head_attributes}>{head}</head><body{body}>{mount}\
         <script data-recreate-entry type=\"module\" src=\"/src/main.jsx\"></script></body></html>"
    )
}

fn head(
    state: &PageState,
    classes: &BTreeMap<String, String>,
    assets: &BTreeMap<String, String>,
) -> String {
    let Some(head) = state.nodes.iter().find(|node| node.tag == "head") else {
        return format!("<title>{}</title>", escape(&state.title));
    };
    state
        .nodes
        .iter()
        .filter(|node| node.parent.as_deref() == Some(head.path.as_str()))
        .filter(|node| super::reemission::safe_head_node(node))
        .map(|node| element(node, state, classes, assets))
        .collect()
}

fn element(
    node: &Node,
    state: &PageState,
    classes: &BTreeMap<String, String>,
    assets: &BTreeMap<String, String>,
) -> String {
    let attributes = attributes(node, classes, assets);
    if matches!(node.tag.as_str(), "base" | "link" | "meta") {
        return format!("<{}{attributes}>", node.tag);
    }

    let text = state
        .nodes
        .iter()
        .filter(|child| child.parent.as_deref() == Some(node.path.as_str()) && child.tag == "#text")
        .map(|child| child.text.as_str())
        .collect::<String>();
    let text = escape(&text);
    format!("<{}{attributes}>{text}</{}>", node.tag, node.tag)
}

/// The one serialiser for every element the emitter writes by hand. `class` and `style` are
/// rebuilt rather than copied: the inline style is replaced by the generated rules, and the
/// authored class tokens are merged with the generated class into a single attribute.
///
/// Every remaining value goes through `asset_attributes::rewrite`, the same call the JSX
/// emitter makes. The shell is the second place captured attributes are written out, so a
/// reference it emits is a reference the artifact advertises; localising here rather than
/// re-deciding which of these attributes name assets keeps that judgement in one owner.
fn attributes(
    node: &Node,
    classes: &BTreeMap<String, String>,
    assets: &BTreeMap<String, String>,
) -> String {
    let mut attributes = node
        .attributes
        .iter()
        .filter(|(name, _)| !matches!(name.as_str(), "class" | "style"))
        .map(|(name, value)| {
            let value = escape(&crate::asset_attributes::rewrite(value, assets));
            if node.tag == "base" && name == "href" {
                return format!(" data-recreate-base-href=\"{value}\"");
            }
            format!(" {name}=\"{value}\"")
        })
        .collect::<String>();
    attributes.push_str(&class_attribute(node, classes));
    attributes
}

/// The generated class carries the element's captured styles and is the only class any
/// emitted element needs. Authored tokens are not merged: nothing in the project selects
/// them, because the emitted stylesheet holds only hashed classes and the definition
/// at-rules `css::global_rule` admits, and `project.rs::root_reset` writes the roots'
/// authored declarations as literal `html`/`body` rules rather than through a token.
fn class_attribute(node: &Node, classes: &BTreeMap<String, String>) -> String {
    let Some(generated) = classes.get(&node.path) else {
        return String::new();
    };
    format!(" class=\"{}\"", escape(generated))
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
