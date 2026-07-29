use super::names;
use crate::model::{Attributes, Node, Rect, Styles};
use std::collections::BTreeMap;

#[test]
fn creates_semantic_component_names() {
    let mut node = Node {
        path: "a".into(),
        parent: None,
        tag: "div".into(),
        text: String::new(),
        attributes: Attributes::new(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        style: Styles::new(),
        before: None,
        after: None,
    };
    node.attributes
        .insert("data-testid".into(), "result-card".into());
    let nodes = BTreeMap::from([(node.path.clone(), &node)]);
    assert_eq!(names::for_node(&node, 0, &nodes), "ResultCard");
}
