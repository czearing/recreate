use super::tree::Components;
use crate::model::{Node, PageState};
use std::collections::BTreeMap;

pub fn for_state(
    base: &Components,
    state: &PageState,
    classes: &BTreeMap<String, String>,
) -> Components {
    for_nodes(base, &state.nodes, classes)
}

pub fn for_nodes(
    base: &Components,
    state_nodes: &[Node],
    classes: &BTreeMap<String, String>,
) -> Components {
    let nodes: BTreeMap<_, _> = state_nodes
        .iter()
        .cloned()
        .map(|node| (node.path.clone(), node))
        .collect();
    Components {
        items: base.items.clone(),
        by_root: base
            .by_root
            .iter()
            .filter(|(path, _)| compatible_root(base, &nodes, classes, path))
            .map(|(path, index)| (path.clone(), *index))
            .collect(),
        children: children(state_nodes),
        classes: classes.clone(),
        nodes,
    }
}

pub fn fragment_nodes(nodes: &[Node], classes: &BTreeMap<String, String>) -> Components {
    Components {
        items: Vec::new(),
        by_root: BTreeMap::new(),
        children: children(nodes),
        classes: classes.clone(),
        nodes: nodes
            .iter()
            .cloned()
            .map(|node| (node.path.clone(), node))
            .collect(),
    }
}

/// A CSS-supplying element is not registered under its parent, so no render walk reaches
/// it. Its rules are captured into `css_rules` and re-emitted through `css::global_rule`
/// like every other authored rule, which is the single route `document::supplies_css`
/// exists to protect: re-serialising the element would apply them a second time at their
/// authored specificity, and a `<style>` in the body would do it just as a head one did.
pub fn children(nodes: &[Node]) -> BTreeMap<String, Vec<String>> {
    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in nodes {
        if super::document::supplies_css(node) {
            continue;
        }
        if let Some(parent) = &node.parent {
            children
                .entry(parent.clone())
                .or_default()
                .push(node.path.clone());
        }
    }
    children
}

fn compatible_root(
    base: &Components,
    nodes: &BTreeMap<String, Node>,
    classes: &BTreeMap<String, String>,
    path: &str,
) -> bool {
    nodes.get(path).is_some_and(|node| {
        base.nodes
            .get(path)
            .is_some_and(|base_node| base_node.tag == node.tag)
    }) && nodes
        .keys()
        .filter(|candidate| {
            candidate.as_str() == path
                || candidate
                    .strip_prefix(path)
                    .is_some_and(|suffix| suffix.starts_with('>'))
        })
        .all(|candidate| base.classes.get(candidate) == classes.get(candidate))
}
