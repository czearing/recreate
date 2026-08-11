use crate::generate::document::render;
use crate::model::{Node, PageState};
use std::collections::BTreeMap;

/// A captured head child. `rect` and `style` are irrelevant to admission, so they are
/// defaulted rather than described, keeping each case's meaningful fields visible.
fn node(path: &str, parent: &str, tag: &str, attributes: &[(&str, &str)]) -> Node {
    serde_json::from_value(serde_json::json!({
        "path": path,
        "parent": parent,
        "tag": tag,
        "text": "",
        "attributes": attributes.iter().copied().collect::<BTreeMap<_, _>>(),
        "rect": {"x": 0.0, "y": 0.0, "width": 0.0, "height": 0.0},
        "style": {},
        "before": null,
        "after": null,
    }))
    .expect("node fixture")
}

/// Drives the real shell emitter, because the defect is a co-occurrence in one emitted
/// file rather than a property of any single element. Every case asserts against the
/// string a browser would actually parse.
fn shell(head_children: Vec<Node>) -> String {
    let mut nodes = vec![
        node("html", "", "html", &[]),
        node("html>head", "html", "head", &[]),
        node("html>body", "html", "body", &[]),
    ];
    nodes.extend(head_children);
    let state = PageState {
        nodes,
        ..PageState::default()
    };
    render(
        Some(&state),
        "<div id=\"root\"></div>",
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
}

/// A head-level `<meta>`, the only shape every case below needs.
fn meta(index: u8, attributes: &[(&str, &str)]) -> Node {
    node(
        &format!("html>head>meta{index}"),
        "html>head",
        "meta",
        attributes,
    )
}

const ENTRY: &str = "<script data-recreate-entry type=\"module\" src=\"/src/main.jsx\"></script>";

/// The conjunction the scene proves. Neither half is a defect alone: a policy in a
/// document that needs nothing is innocent, and an entry script in a document with no
/// policy is innocent. `render` writes the head ahead of the entry script in one
/// `format!`, and a meta-delivered policy governs everything that follows it, so a
/// captured `script-src 'none'` forbids the artifact's own entry point.
#[test]
fn refuses_a_pragma_that_would_forbid_the_entry_script() {
    let shell = shell(vec![meta(
        1,
        &[
            ("http-equiv", "Content-Security-Policy"),
            ("content", "script-src 'none'"),
        ],
    )]);
    assert!(
        !shell.contains("http-equiv"),
        "the pragma was re-emitted into the shell: {shell}"
    );
    assert!(
        !shell.contains("script-src"),
        "the pragma's directive survived without its attribute: {shell}"
    );
    assert!(
        shell.contains(ENTRY),
        "the entry script must be present and unchanged: {shell}"
    );
}

/// The inverse guard. Correct behaviour here is an absence, so a fix that drops every
/// `<meta>` would pass the subject while destroying the descriptive metadata other
/// behaviour depends on. `charset` and `name` are separate attributes in the spec's own
/// partition and carry no instruction.
#[test]
fn keeps_the_encoding_declaration_and_descriptive_metadata() {
    let shell = shell(vec![
        meta(1, &[("charset", "utf-8")]),
        meta(2, &[("name", "description"), ("content", "KEEPMETATOKEN")]),
        meta(
            3,
            &[
                ("name", "viewport"),
                ("content", "width=device-width,initial-scale=1"),
            ],
        ),
    ]);
    assert!(shell.contains("charset=\"utf-8\""), "{shell}");
    assert!(shell.contains("KEEPMETATOKEN"), "{shell}");
    assert!(
        shell.contains("width=device-width,initial-scale=1"),
        "{shell}"
    );
}

/// Kills a deny-list implementation. The pragma set is open and versioned, so a list of
/// hazardous directives is short the day it is written; the last case names a directive
/// no list could anticipate and must be refused on the same evidence as the first two.
#[test]
fn refuses_every_pragma_including_one_no_list_anticipates() {
    for (directive, content) in [
        ("refresh", "0; url=https://example.com/"),
        (
            "Content-Security-Policy-Report-Only",
            "script-src 'nonce-x'",
        ),
        ("x-unheard-of-directive", "UNANTICIPATEDPRAGMA"),
    ] {
        let shell = shell(vec![meta(
            1,
            &[("http-equiv", directive), ("content", content)],
        )]);
        assert!(
            !shell.contains(directive) && !shell.contains(content),
            "{directive} survived into the shell: {shell}"
        );
    }
}

/// Per-element refusal. A fix that rejects the whole head, or aborts, on meeting a
/// directive it cannot model turns a silent loss into a hard failure.
#[test]
fn refuses_only_the_pragma_and_keeps_the_rest_of_the_head() {
    let mut text = node("html>head>title>#text", "html>head>title", "#text", &[]);
    text.text = "KEPTTITLE".into();
    let shell = shell(vec![
        meta(1, &[("http-equiv", "refresh"), ("content", "0")]),
        meta(2, &[("name", "description"), ("content", "KEEPMETATOKEN")]),
        node("html>head>title", "html>head", "title", &[]),
        text,
    ]);
    assert!(!shell.contains("http-equiv"), "{shell}");
    assert!(shell.contains("KEEPMETATOKEN"), "{shell}");
    assert!(
        shell.contains("<title>KEPTTITLE</title>"),
        "the head was rejected wholesale: {shell}"
    );
}

/// The same rule must reach the second emitter. A stray `<meta>` the parser left in the
/// body never goes through the shell at all — it is registered as a render child and
/// written into a component, where React hoists it back into the live `<head>`. One rule
/// enforced on one emitter is the defect that keeps recurring, so `children` consults the
/// same owner it already consults for CSS delivery.
#[test]
fn refuses_a_body_pragma_the_render_walk_would_hoist_into_the_head() {
    let registered = crate::generate::structural_tree::children(&[
        node("html>body", "html", "body", &[]),
        node(
            "html>body>meta",
            "html>body",
            "meta",
            &[("http-equiv", "refresh"), ("content", "0; url=/elsewhere")],
        ),
    ]);
    assert_eq!(
        registered.get("html>body").map(Vec::as_slice),
        None,
        "the body pragma was registered for the render walk"
    );
}

/// The inverse guard for the second emitter: a descriptive `<meta>` in the body is
/// ordinary content and must still render.
#[test]
fn keeps_a_descriptive_body_meta_in_the_render_walk() {
    let registered = crate::generate::structural_tree::children(&[
        node("html>body", "html", "body", &[]),
        node(
            "html>body>meta",
            "html>body",
            "meta",
            &[("itemprop", "position"), ("content", "1")],
        ),
    ]);
    assert_eq!(
        registered["html>body"],
        vec!["html>body>meta".to_string()],
        "a descriptive body meta must still be rendered"
    );
}
