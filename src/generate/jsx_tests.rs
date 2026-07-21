use super::*;
use crate::model::{Node, Rect, Styles};

fn text(path: &str, parent: &str, value: &str, x: f64, width: f64) -> Node {
    Node {
        path: path.into(),
        parent: Some(parent.into()),
        tag: "#text".into(),
        text: value.into(),
        attributes: BTreeMap::new(),
        rect: Rect {
            x,
            y: 10.0,
            width,
            height: 20.0,
        },
        style: Styles::new(),
        before: None,
        after: None,
    }
}

#[test]
fn preserves_visible_gap_between_adjacent_text_nodes() {
    let parent = "html>body:nth-of-type(1)>div:nth-of-type(1)";
    let first = text(&format!("{parent}>#text(1)"), parent, "0", 10.0, 8.0);
    let second = text(&format!("{parent}>#text(2)"), parent, "items", 22.0, 34.0);
    let components = Components {
        items: Vec::new(),
        by_root: BTreeMap::new(),
        children: BTreeMap::from([(parent.into(), vec![first.path.clone(), second.path.clone()])]),
        classes: BTreeMap::new(),
        nodes: BTreeMap::from([(first.path.clone(), first), (second.path.clone(), second)]),
    };

    let output = render_children(parent, &components, &BTreeMap::new(), 1, &BTreeMap::new());

    assert!(output.contains("{\"0\"}\n  {\" \"}\n  {\"items\"}"));
}
