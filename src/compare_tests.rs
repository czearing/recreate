use crate::{
    compare_node::{compare, same_rect},
    model::{Attributes, Node, PageState, Rect, Styles, Viewport},
};

fn node(path: &str, x: f64) -> Node {
    Node {
        path: path.into(),
        parent: None,
        tag: "div".into(),
        text: "value".into(),
        attributes: Attributes::new(),
        rect: Rect {
            x,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        },
        style: Styles::new(),
        before: None,
        after: None,
    }
}

#[test]
fn geometry_tolerance_is_bounded() {
    assert!(same_rect(&node("a", 0.0), &node("a", 1.0)));
    assert!(same_rect(&node("a", 0.0), &node("a", 1.5078125)));
    assert!(!same_rect(&node("a", 0.0), &node("a", 1.52)));
    assert!(!same_rect(&node("a", 0.0), &node("a", 2.0)));
}

#[test]
fn rejects_zero_area_geometry_changes() {
    let mut expected = node("a", 0.0);
    expected.rect.height = 0.0;
    let mut actual = node("a", 100.0);
    actual.rect.width = 1000.0;
    assert!(!same_rect(&expected, &actual));
}

#[test]
fn rejects_missing_and_unexpected_nodes_after_conversion() {
    let expected = state(vec![node("html", 0.0), node("html>body", 0.0)]);
    let actual = state(vec![node("html", 0.0), node("html>main", 0.0)]);
    let report = compare(&expected, &actual);
    assert_eq!(report.missing, 1);
    assert_eq!(report.unexpected, 1);
}

#[test]
fn accepts_rewritten_asset_urls_with_identical_bytes() {
    let mut expected_node = node("html>body>img", 0.0);
    expected_node.tag = "img".into();
    expected_node
        .attributes
        .insert("src".into(), "blob:https://example.test/avatar".into());
    let mut actual_node = expected_node.clone();
    actual_node
        .attributes
        .insert("src".into(), "/assets/avatar.jpg".into());
    let mut expected = state(vec![expected_node]);
    let mut actual = state(vec![actual_node]);
    expected.asset_data.insert(
        "blob:https://example.test/avatar".into(),
        "same-bytes".into(),
    );
    actual.asset_data.insert(
        "http://127.0.0.1:4173/assets/avatar.jpg".into(),
        "same-bytes".into(),
    );

    assert_eq!(compare(&expected, &actual).attribute_mismatches, 0);
}

fn state(nodes: Vec<Node>) -> PageState {
    PageState {
        url: "https://example.test".into(),
        title: "Exact".into(),
        viewport: Viewport::default(),
        dom: Default::default(),
        capture_blockers: Vec::new(),
        nodes,
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}
