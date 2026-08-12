use super::names;
use crate::model::{Attributes, Node, Rect, Styles};
use std::collections::BTreeMap;

#[test]
fn creates_semantic_component_names() {
    let mut node = Node {
        writing_mode: Default::default(),
        blocking_overlay: false,
        disabled: false,
        rtl: false,
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

#[test]
fn prefers_the_source_component_name_over_page_copy() {
    let mut node = Node {
        writing_mode: Default::default(),
        blocking_overlay: false,
        disabled: false,
        rtl: false,
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
    node.attributes.insert(
        "aria-label".into(),
        "Create weekly planning assistant".into(),
    );
    node.attributes.insert("role".into(), "presentation".into());
    let nodes = BTreeMap::from([(node.path.clone(), &node)]);
    assert_eq!(names::for_node(&node, 0, &nodes), "Presentation");

    node.attributes.insert(
        "class".into(),
        "src-NotebookCard-NotebookCard-module__createCard-fs8I4M".into(),
    );
    let nodes = BTreeMap::from([(node.path.clone(), &node)]);
    assert_eq!(names::for_node(&node, 0, &nodes), "NotebookCard");
}
