use crate::model::{Node, PageState};
use std::collections::BTreeMap;

pub fn render(
    state: Option<&PageState>,
    mount: &str,
    classes: &BTreeMap<String, String>,
) -> String {
    let html = state
        .and_then(|state| state.nodes.iter().find(|node| node.tag == "html"))
        .map(|node| attributes(node, classes))
        .unwrap_or_default();
    let body = state
        .and_then(|state| state.nodes.iter().find(|node| node.tag == "body"))
        .map(|node| attributes(node, classes))
        .unwrap_or_default();
    let head_attributes = state
        .and_then(|state| state.nodes.iter().find(|node| node.tag == "head"))
        .map(|node| attributes(node, classes))
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
        .filter(|node| safe_head_node(node))
        .map(|node| element(node, state, classes))
        .collect()
}

fn safe_head_node(node: &Node) -> bool {
    if supplies_css(node) {
        return false;
    }
    if node.tag != "link" {
        return matches!(node.tag.as_str(), "base" | "meta" | "title");
    }
    let relation = node.attributes.get("rel").map(String::as_str);
    let kind = node.attributes.get("as").map(String::as_str);
    let href = node.attributes.get("href").map(String::as_str);
    safe_link(relation, kind) && resolvable_link(href)
}

/// Whether an element delivers authored CSS, by any route.
///
/// Every authored rule already reaches the output through `css_base`, which re-emits from
/// the captured `css_rules` under `css::global_rule` — the gate that exists because the
/// bake already represents any rule that reached an element through a selector. Emitting
/// a `<style>` verbatim, or leaving a stylesheet `<link>` live, routes around that gate
/// and applies those rules a second time, at whatever specificity they were authored
/// with. So which declaration wins would depend on how the page happened to ship it, and
/// an authored rule the original cascade rejected can outrank the value that beat it.
/// Rejecting the delivery instead of filtering its text keeps one owner for the decision.
pub(super) fn supplies_css(node: &Node) -> bool {
    node.tag == "style"
        || (node.tag == "link"
            && node.attributes.get("rel").is_some_and(|relation| {
                relation
                    .split_ascii_whitespace()
                    .any(|token| token.eq_ignore_ascii_case("stylesheet"))
            }))
}

/// A relative href names a file in the source site's own build output, which the
/// recreation never produces, so keeping it only yields a guaranteed 404. Absolute
/// and data hrefs still resolve, so they are kept.
fn resolvable_link(href: Option<&str>) -> bool {
    let Some(href) = href.map(str::trim) else {
        return false;
    };
    href.starts_with("data:") || href.starts_with("//") || {
        let scheme = href.split_once("://").map(|(scheme, _)| scheme);
        matches!(scheme, Some("http") | Some("https"))
    }
}

fn safe_link(relation: Option<&str>, kind: Option<&str>) -> bool {
    relation != Some("modulepreload") && !(relation == Some("preload") && kind == Some("script"))
}

fn element(node: &Node, state: &PageState, classes: &BTreeMap<String, String>) -> String {
    let attributes = attributes(node, classes);
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
fn attributes(node: &Node, classes: &BTreeMap<String, String>) -> String {
    let mut attributes = node
        .attributes
        .iter()
        .filter(|(name, _)| !matches!(name.as_str(), "class" | "style"))
        .map(|(name, value)| {
            if node.tag == "base" && name == "href" {
                return format!(" data-recreate-base-href=\"{}\"", escape(value));
            }
            format!(" {name}=\"{}\"", escape(value))
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

#[cfg(test)]
mod tests {
    use super::{resolvable_link, safe_link};

    #[test]
    fn excludes_executable_source_preloads() {
        assert!(!safe_link(Some("modulepreload"), None));
        assert!(!safe_link(Some("preload"), Some("script")));
        assert!(safe_link(Some("stylesheet"), None));
        assert!(safe_link(Some("icon"), None));
    }

    /// A relative href names the source site's build output, so the recreation
    /// would request a file it never generates and the browser logs a 404.
    #[test]
    fn excludes_links_to_the_source_projects_own_files() {
        assert!(!resolvable_link(Some("./assets/index-BIZnfT4P.css")));
        assert!(!resolvable_link(Some("./onenote-favicon.svg")));
        assert!(!resolvable_link(Some("/static/app.css")));
        assert!(!resolvable_link(None));
    }

    #[test]
    fn keeps_links_that_still_resolve() {
        assert!(resolvable_link(Some("https://fonts.example.com/font.css")));
        assert!(resolvable_link(Some("http://cdn.example.com/a.css")));
        assert!(resolvable_link(Some("//cdn.example.com/a.css")));
        assert!(resolvable_link(Some("data:,")));
    }
}
