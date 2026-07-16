use crate::{
    compare::same_rect,
    model::{Attributes, Node, Rect, Styles},
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
