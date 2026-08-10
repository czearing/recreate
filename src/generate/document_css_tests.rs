use super::document;
use crate::model::PageState;
use std::collections::BTreeMap;

/// Builds a head that ships the same authored CSS by a chosen delivery mechanism. The
/// stylesheet's rules are already in `css_rules` whichever route is used, because the
/// capture walks `document.styleSheets`, so the mechanism is the only variable.
fn page(delivery: Delivery) -> PageState {
    let element = |path: &str, parent: &str, tag: &str, attributes: serde_json::Value| {
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
    let head = "html>head:nth-of-type(1)";
    let mut nodes = vec![
        element("html", "", "html", serde_json::json!({"lang": "en"})),
        element(head, "html", "head", serde_json::json!({})),
        element("html>body:nth-of-type(1)", "html", "body", serde_json::json!({})),
    ];
    let sheet = "#hero{color:rgb(200,0,0)}";
    match delivery {
        Delivery::Inline => {
            nodes.push(element(
                "style",
                head,
                "style",
                serde_json::json!({"type": "text/css"}),
            ));
            nodes.push(serde_json::json!({
                "path": "style>text", "parent": "style", "tag": "#text", "text": sheet,
                "attributes": {}, "rect": {"x": 0.0, "y": 0.0, "width": 0.0, "height": 0.0}, "style": {},
            }));
        }
        Delivery::RelativeLink | Delivery::AbsoluteLink => {
            let href = match delivery {
                Delivery::RelativeLink => "./over.css",
                _ => "https://cdn.example.test/over.css",
            };
            nodes.push(element(
                "link",
                head,
                "link",
                serde_json::json!({"rel": "stylesheet", "href": href}),
            ));
        }
    }
    nodes.push(element(
        "title",
        head,
        "title",
        serde_json::json!({}),
    ));
    nodes.push(serde_json::json!({
        "path": "title>text", "parent": "title", "tag": "#text", "text": "Scene",
        "attributes": {}, "rect": {"x": 0.0, "y": 0.0, "width": 0.0, "height": 0.0}, "style": {},
    }));
    serde_json::from_value(serde_json::json!({
        "url": "https://example.test/",
        "title": "Scene",
        "nodes": nodes,
        "animations": [],
        "css_rules": [sheet],
        "asset_urls": [],
    }))
    .unwrap()
}

#[derive(Clone, Copy)]
enum Delivery {
    Inline,
    RelativeLink,
    AbsoluteLink,
}

fn render(delivery: Delivery) -> String {
    document::render(Some(&page(delivery)), "", &BTreeMap::new())
}

/// The tool bakes each element's computed style into one class, so an authored rule the
/// head re-supplies is applied a second time — and an id selector at 1-0-0 outranks that
/// class at 0-1-0, repainting whatever the original cascade had chosen instead.
#[test]
fn drops_authored_css_delivered_by_a_head_style_element() {
    let html = render(Delivery::Inline);
    assert!(!html.contains("<style"), "was {html}");
    assert!(!html.contains("rgb(200,0,0)"), "was {html}");
}

/// An absolute href was kept as a live `<link>`, so the recreation refetched a third-party
/// sheet at view time and applied rules the bake already carries.
#[test]
fn drops_authored_css_delivered_by_a_stylesheet_link() {
    for delivery in [Delivery::RelativeLink, Delivery::AbsoluteLink] {
        let html = render(delivery);
        assert!(!html.contains("stylesheet"), "was {html}");
        assert!(!html.contains("over.css"), "was {html}");
    }
}

/// The invariant itself: two pages shipping byte-identical CSS by different routes render
/// identically, so their recreations must be identical too.
#[test]
fn emits_the_same_document_however_the_css_was_delivered() {
    let inline = render(Delivery::Inline);
    assert_eq!(inline, render(Delivery::RelativeLink));
    assert_eq!(inline, render(Delivery::AbsoluteLink));
}

/// Positive control: head elements that carry no CSS still survive, so the fix removed a
/// delivery route rather than the head walk.
#[test]
fn keeps_head_elements_that_carry_no_authored_css() {
    let html = render(Delivery::Inline);
    assert!(html.contains("<title>Scene</title>"), "was {html}");
}

/// The body is the fourth delivery route, and a `<style>` there reaches the output through
/// the JSX tree rather than the head walk, so the predicate has to gate both.
#[test]
fn drops_authored_css_delivered_from_the_body() {
    let nodes = page(Delivery::Inline).nodes;
    let mut style = nodes
        .iter()
        .find(|node| node.tag == "style")
        .expect("the fixture ships a style element")
        .clone();
    style.parent = Some("html>body:nth-of-type(1)".into());
    let children = super::structural_tree::children(&[style]);
    assert!(children.is_empty(), "was {children:?}");
}
