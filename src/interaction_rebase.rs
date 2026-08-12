use crate::interaction_rebase_node::rebase_node;
use crate::model::{Node, PageState};
use std::collections::BTreeMap;

pub fn unchanged(state: &mut PageState, fresh: &PageState, baseline: &PageState) {
    let fresh: BTreeMap<_, _> = fresh
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    // A path is a chain of `:nth-of-type()` ordinals, so a departed sibling shifts every
    // following same-tag path up one and the same path names a different element in the
    // two captures. The alignment is the single owner of "which baseline node is this",
    // and asking it here keeps the rebase from writing the departed sibling's values over
    // its successor. A node with no counterpart is one the interaction introduced, and an
    // introduced node has no baseline value to be restored to.
    let counterparts: Vec<Option<&Node>> = {
        let alignment = crate::node_alignment::of(state, baseline);
        state
            .nodes
            .iter()
            .map(|node| alignment.counterpart(&node.path))
            .collect()
    };
    for (node, baseline) in state.nodes.iter_mut().zip(counterparts) {
        let (Some(fresh), Some(baseline)) = (fresh.get(node.path.as_str()), baseline) else {
            continue;
        };
        rebase_node(node, fresh, baseline);
    }
}

pub fn causal(state: &mut PageState, before: &PageState, affected: &[String]) {
    let before_by_path: BTreeMap<_, _> = before
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    let related = |path: &str| {
        affected.iter().any(|root| {
            path == root
                || path
                    .strip_prefix(root)
                    .is_some_and(|suffix| suffix.starts_with('>'))
        })
    };
    state
        .nodes
        .retain(|node| before_by_path.contains_key(node.path.as_str()) || related(&node.path));
    for node in &mut state.nodes {
        if related(&node.path) {
            continue;
        }
        if let Some(original) = before_by_path.get(node.path.as_str()) {
            node.clone_from(original);
        }
    }
    state
        .dom
        .retain(|path, _| before.dom.contains_key(path) || related(path));
    for (path, value) in &mut state.dom {
        if related(path) {
            continue;
        }
        if let Some(original) = before.dom.get(path) {
            // The causal scope is assembled from MutationObserver records, and scrolling an
            // element mutates nothing: no record is emitted, so a scrolled element can never
            // enter the scope however it is styled. Rebasing its offset here would delete the
            // one fact only the capture can supply, so the recorded offsets are carried over
            // while every measured field is restored.
            let (left, top) = (value.scroll_left, value.scroll_top);
            value.clone_from(original);
            value.scroll_left = left;
            value.scroll_top = top;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::causal;
    use crate::interaction_rebase_node::rebase_map;
    use crate::model::{DomNode, Node, PageState, Rect, Viewport};
    use std::collections::BTreeMap;

    #[test]
    fn preserves_only_live_interaction_changes() {
        let mut state = BTreeMap::from([
            ("color".into(), "blue".into()),
            ("margin".into(), "8px".into()),
        ]);
        let fresh = BTreeMap::from([
            ("color".into(), "red".into()),
            ("margin".into(), "8px".into()),
        ]);
        let baseline = BTreeMap::from([
            ("color".into(), "black".into()),
            ("margin".into(), "0px".into()),
        ]);
        rebase_map(&mut state, &fresh, &baseline);
        assert_eq!(state["color"], "blue");
        assert_eq!(state["margin"], "0px");
    }

    fn node(path: &str, text: &str) -> Node {
        Node {
            writing_mode: Default::default(),
            blocking_overlay: false,
            path: path.into(),
            parent: Some("html>body".into()),
            tag: "div".into(),
            text: text.into(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            style: Default::default(),
            before: None,
            after: None,
            disabled: false,
            rtl: false,
        }
    }

    fn state(nodes: Vec<Node>) -> PageState {
        PageState {
            url: String::new(),
            title: String::new(),
            viewport: Viewport::default(),
            nodes,
            dom: Default::default(),
            capture_blockers: Vec::new(),
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

    /// Scrolling emits no MutationObserver record, so a scrolled element can never enter the
    /// causal scope. Rebasing its recorded offset away is what deleted interaction scroll.
    #[test]
    fn causal_scope_keeps_recorded_scroll_of_unrelated_elements() {
        let dom = |top: f64| DomNode {
            scroll_top: top,
            client_height: 240.0,
            ..DomNode::default()
        };
        let before = {
            let mut state = state(vec![node("html>body>button", "before")]);
            state.dom.insert("html>body>panel".into(), dom(0.0));
            state
        };
        let mut after = {
            let mut state = state(vec![node("html>body>button", "after")]);
            state.dom.insert("html>body>panel".into(), dom(300.0));
            state
        };
        causal(&mut after, &before, &["html>body>button".into()]);
        assert_eq!(after.dom["html>body>panel"].scroll_top, 300.0);
    }

    #[test]
    fn causal_scope_restores_unrelated_nodes() {
        let before = state(vec![
            node("html>body>button", "before"),
            node("html>body>aside", "stable"),
        ]);
        let mut after = state(vec![
            node("html>body>button", "after"),
            node("html>body>aside", "spontaneous"),
        ]);
        causal(&mut after, &before, &["html>body>button".into()]);
        assert_eq!(after.nodes[0].text, "after");
        assert_eq!(after.nodes[1].text, "stable");
    }
}
