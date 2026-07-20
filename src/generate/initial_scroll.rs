use crate::model::{Node, PageState, Specification};

pub fn targets(specification: &Specification) -> String {
    serde_json::to_string(
        &specification
            .states
            .iter()
            .map(snapshot)
            .collect::<Vec<_>>(),
    )
    .expect("initial scroll targets should serialize")
}

fn snapshot(state: &PageState) -> Vec<(&str, i64, i64)> {
    state
        .nodes
        .iter()
        .filter_map(|parent| scroll_offset(parent, state).map(|(x, y)| (&*parent.path, x, y)))
        .collect()
}

fn scroll_offset(parent: &Node, state: &PageState) -> Option<(i64, i64)> {
    if parent.rect.width <= 0.0 || parent.rect.height <= 0.0 {
        return None;
    }
    let first = state
        .nodes
        .iter()
        .find(|node| node.tag != "#text" && node.parent.as_deref() == Some(&parent.path))?;
    let left = if scrollable(parent, "overflow-x") {
        (parent.rect.x - first.rect.x).max(0.0).round() as i64
    } else {
        0
    };
    let top = if scrollable(parent, "overflow-y") {
        (parent.rect.y - first.rect.y).max(0.0).round() as i64
    } else {
        0
    };
    (left > 1 || top > 1).then_some((left, top))
}

fn scrollable(node: &Node, property: &str) -> bool {
    node.style
        .get(property)
        .is_some_and(|value| matches!(value.as_str(), "auto" | "scroll"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Rect, Viewport};

    fn node(path: &str, parent: Option<&str>, y: f64, overflow: &str) -> Node {
        Node {
            path: path.into(),
            parent: parent.map(str::to_string),
            tag: "div".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y,
                width: 300.0,
                height: 200.0,
            },
            style: [("overflow-y".into(), overflow.into())].into(),
            before: None,
            after: None,
        }
    }

    fn state(nodes: Vec<Node>) -> PageState {
        PageState {
            url: String::new(),
            title: String::new(),
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

    #[test]
    fn derives_internal_scroll_from_first_child_geometry() {
        let parent = node("parent", None, 48.0, "auto");
        let child = node("parent>child", Some("parent"), -112.0, "visible");
        assert_eq!(
            snapshot(&state(vec![parent, child])),
            vec![("parent", 0, 160)]
        );
    }

    #[test]
    fn ignores_visible_and_unshifted_containers() {
        let visible = node("visible", None, 0.0, "visible");
        let child = node("visible>child", Some("visible"), -40.0, "visible");
        let still = node("still", None, 0.0, "auto");
        let still_child = node("still>child", Some("still"), 0.0, "visible");
        assert!(snapshot(&state(vec![visible, child, still, still_child])).is_empty());
    }
}
