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
    use super::rebase_map;
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
}
