//! What the index reads out of an authored stylesheet.

use super::Index;
use crate::model::{Node, Rect, Styles};

fn node() -> Node {
    let mut node = Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: "button".into(),
        parent: None,
        tag: "button".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        },
        style: Styles::from([("display".into(), "flex".into())]),
        ..Default::default()
    };
    node.attributes
        .insert("class".into(), "primary control".into());
    node.attributes.insert("data-kind".into(), "action".into());
    node
}

#[test]
fn indexes_class_tag_and_attribute_selectors_without_changing_order() {
    let rules = vec![
        ".primary{display:block;width:40px;}".into(),
        "button[data-kind=\"action\"]{display:flex;width:50px;}".into(),
    ];
    let styles = Index::new(&rules).declarations(&node());
    assert_eq!(styles["display"], "flex");
    assert_eq!(styles["width"], "50px");
}

#[test]
fn indexes_universal_selectors_for_every_node() {
    let rules = vec!["*{width:40px;}".into()];
    let index = Index::new(&rules);
    assert_eq!(index.table.universal, vec![0]);
    assert_eq!(index.direct_indices(&node()), vec![0]);
    assert_eq!(index.declarations(&node())["width"], "40px");
}
