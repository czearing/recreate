use super::tree::Components;
use crate::model::{Node, PageState};
use std::collections::BTreeMap;

pub fn for_state(
    base: &Components,
    state: &PageState,
    classes: &BTreeMap<String, String>,
) -> Components {
    let nodes: BTreeMap<_, _> = state
        .nodes
        .iter()
        .cloned()
        .map(|node| (node.path.clone(), node))
        .collect();
    Components {
        items: base.items.clone(),
        by_root: base
            .by_root
            .iter()
            .filter(|(path, _)| compatible_root(base, &nodes, path))
            .map(|(path, index)| (path.clone(), *index))
            .collect(),
        children: children(&state.nodes),
        classes: classes.clone(),
        nodes,
    }
}

pub fn children(nodes: &[Node]) -> BTreeMap<String, Vec<String>> {
    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in nodes {
        if let Some(parent) = &node.parent {
            children
                .entry(parent.clone())
                .or_default()
                .push(node.path.clone());
        }
    }
    children
}

fn compatible_root(base: &Components, nodes: &BTreeMap<String, Node>, path: &str) -> bool {
    nodes.get(path).is_some_and(|node| {
        base.nodes
            .get(path)
            .is_some_and(|base_node| base_node.tag == node.tag)
    })
}
