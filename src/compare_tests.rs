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
    assert!(!same_rect(&node("a", 0.0), &node("a", 2.0)));
}

#[test]
fn ignores_zero_area_wrapper_geometry() {
    let mut expected = node("a", 0.0);
    expected.rect.height = 0.0;
    let mut actual = node("a", 100.0);
    actual.rect.width = 1000.0;
    assert!(same_rect(&expected, &actual));
}

#[test]
fn rejects_missing_and_unexpected_nodes_after_conversion() {
    let expected = state(vec![node("html", 0.0), node("html>body", 0.0)]);
    let actual = state(vec![node("html", 0.0), node("html>main", 0.0)]);
    let report = compare(&expected, &actual);
    assert_eq!(report.missing, 1);
    assert_eq!(report.unexpected, 1);
}

fn state(nodes: Vec<Node>) -> PageState {
    PageState {
        url: "https://example.test".into(),
        title: "Exact".into(),
        viewport: Viewport::default(),
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
