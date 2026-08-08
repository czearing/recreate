use super::document;
use crate::model::PageState;
use std::collections::BTreeMap;

/// Builds a page whose document root carries an authored scoping token alongside an
/// attribute of the same shape, so the only variable is which attribute carries the scope.
fn page(html_class: Option<&str>, body_class: Option<&str>) -> PageState {
    let node = |path: &str, parent: Option<&str>, tag: &str, class: Option<&str>| {
        let mut attributes = serde_json::Map::new();
        attributes.insert("lang".into(), "en".into());
        if tag == "html" {
            attributes.insert("data-theme".into(), "dark".into());
            attributes.insert("style".into(), "color:red".into());
        }
        if let Some(class) = class {
            attributes.insert("class".into(), class.into());
        }
        serde_json::json!({
            "path": path,
            "parent": parent,
            "tag": tag,
            "text": "",
            "attributes": attributes,
            "rect": {"x": 0.0, "y": 0.0, "width": 800.0, "height": 600.0},
            "style": {},
        })
    };
    serde_json::from_value(serde_json::json!({
        "url": "https://example.test/",
        "title": "Root scope token",
        "nodes": [
            node("html", None, "html", html_class),
            node("html>head:nth-of-type(1)", Some("html"), "head", None),
            node("html>body:nth-of-type(1)", Some("html"), "body", body_class),
        ],
        "animations": [],
        "css_rules": [],
        "asset_urls": [],
    }))
    .unwrap()
}

fn classes() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("html".to_string(), "rhtml".to_string()),
        ("html>body:nth-of-type(1)".to_string(), "rbody".to_string()),
    ])
}

fn start_tag(html: &str, tag: &str) -> String {
    let open = format!("<{tag}");
    let start = html.find(&open).expect("start tag is emitted");
    let end = html[start..].find('>').expect("start tag is closed") + start;
    html[start..=end].to_string()
}

/// The authored token qualifies every root-scoped rule in the re-emitted stylesheet, so
/// deleting it renders those rules inert while leaving them greppable in the output.
#[test]
fn keeps_the_authored_scoping_token_on_the_document_root() {
    let html = document::render(Some(&page(Some("dark"), Some("theme-a"))), "", &classes());
    let root = start_tag(&html, "html");
    assert!(root.contains("dark"), "html start tag was {root}");
    let body = start_tag(&html, "body");
    assert!(body.contains("theme-a"), "body start tag was {body}");
}

/// The generated class is the document root's only route to its own captured styles,
/// because html and body are never rendered as components.
#[test]
fn binds_the_generated_class_to_the_document_root() {
    let html = document::render(Some(&page(Some("dark"), None)), "", &classes());
    assert!(start_tag(&html, "html").contains("rhtml"));
    assert!(start_tag(&html, "body").contains("rbody"));
}

/// Two class attributes on one tag is invalid markup and the browser keeps only the first,
/// so the authored tokens and the generated class must share a single attribute.
#[test]
fn emits_exactly_one_class_attribute_per_root_tag() {
    let html = document::render(Some(&page(Some("dark"), Some("theme-a"))), "", &classes());
    for tag in ["html", "body"] {
        assert_eq!(
            start_tag(&html, tag).matches("class=").count(),
            1,
            "{tag} start tag was {}",
            start_tag(&html, tag)
        );
    }
}

/// A root with no authored class still needs its generated class, which is the job the
/// runtime selector-rewrite used to do.
#[test]
fn binds_the_generated_class_when_nothing_was_authored() {
    let html = document::render(Some(&page(None, None)), "", &classes());
    assert!(start_tag(&html, "html").contains("class=\"rhtml\""));
    assert!(start_tag(&html, "body").contains("class=\"rbody\""));
}

/// Positive control and negative control in one assertion: every other attribute passes
/// through untouched, while the inline style is still replaced by the generated rules.
#[test]
fn passes_other_root_attributes_through_and_still_drops_inline_style() {
    let html = document::render(Some(&page(Some("dark"), None)), "", &classes());
    let root = start_tag(&html, "html");
    assert!(root.contains("data-theme=\"dark\""), "was {root}");
    assert!(root.contains("lang=\"en\""), "was {root}");
    assert!(!root.contains("style="), "was {root}");
}
