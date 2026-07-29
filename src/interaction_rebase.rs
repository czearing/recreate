use crate::model::{Node, PageState, Pseudo, Styles};
use std::collections::{BTreeMap, BTreeSet};

pub fn unchanged(state: &mut PageState, fresh: &PageState, baseline: &PageState) {
    let fresh: BTreeMap<_, _> = fresh
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    let baseline: BTreeMap<_, _> = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    for node in &mut state.nodes {
        let Some(fresh) = fresh.get(node.path.as_str()) else {
            continue;
        };
        let Some(baseline) = baseline.get(node.path.as_str()) else {
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
            value.clone_from(original);
        }
    }
}

fn rebase_node(state: &mut Node, fresh: &Node, baseline: &Node) {
    if state.text == fresh.text {
        state.text.clone_from(&baseline.text);
    }
    if state.rect == fresh.rect {
        state.rect.clone_from(&baseline.rect);
    }
    rebase_map(
        &mut state.attributes,
        &fresh.attributes,
        &baseline.attributes,
    );
    rebase_map(&mut state.style, &fresh.style, &baseline.style);
    rebase_pseudo(&mut state.before, &fresh.before, &baseline.before);
    rebase_pseudo(&mut state.after, &fresh.after, &baseline.after);
}

fn rebase_pseudo(state: &mut Option<Pseudo>, fresh: &Option<Pseudo>, baseline: &Option<Pseudo>) {
    let (Some(state), Some(fresh), Some(baseline)) = (state, fresh, baseline) else {
        return;
    };
    if state.content == fresh.content {
        state.content.clone_from(&baseline.content);
    }
    rebase_map(&mut state.style, &fresh.style, &baseline.style);
}

fn rebase_map(state: &mut Styles, fresh: &Styles, baseline: &Styles) {
    let keys: BTreeSet<_> = state.keys().chain(fresh.keys()).cloned().collect();
    for key in keys {
        if state.get(&key) != fresh.get(&key) {
            continue;
        }
        match baseline.get(&key) {
            Some(value) => {
                state.insert(key, value.clone());
            }
            None => {
                state.remove(&key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{causal, rebase_map};
    use crate::model::{Node, PageState, Rect, Viewport};
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

    #[test]
    fn causal_scope_restores_unrelated_nodes() {
        let node = |path: &str, text: &str| Node {
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
        };
        let state = |nodes| PageState {
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
        };
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
