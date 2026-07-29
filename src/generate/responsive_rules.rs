use super::{
    flex::{constrained_by_flex_chain, shrunk_flex_item},
    node_rules::append_node_rules,
};
use crate::model::{Node, Specification};
use std::collections::{BTreeMap, HashMap, HashSet};

pub fn append_filtered(
    specification: &Specification,
    assets: &BTreeMap<String, String>,
    classes: &BTreeMap<String, String>,
    css: &mut String,
    paths: Option<&HashSet<String>>,
    fluid_heights: &HashSet<String>,
) {
    let Some(base) = specification.states.first() else {
        return;
    };
    let base_nodes: HashMap<_, _> = base
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    let mut states: Vec<_> = specification.states.iter().skip(1).collect();
    states.sort_by_key(|state| std::cmp::Reverse(state.viewport.width));

    for (index, state) in states.iter().enumerate() {
        let state_nodes: HashMap<_, _> = state
            .nodes
            .iter()
            .map(|node| (node.path.as_str(), node))
            .collect();
        let shrunk_roots = shrunk_roots(&state.nodes, &base_nodes, &state_nodes);
        let mut rules = String::new();
        for node in &state.nodes {
            if paths.is_some_and(|paths| !paths.contains(&node.path)) {
                continue;
            }
            let (Some(base_node), Some(class)) =
                (base_nodes.get(node.path.as_str()), classes.get(&node.path))
            else {
                continue;
            };
            rules.push_str(&append_node_rules(
                base_node,
                node,
                node.parent
                    .as_deref()
                    .and_then(|parent| state_nodes.get(parent).copied()),
                (&base.viewport, &state.viewport),
                class,
                assets,
                &state.css_rules,
                fluid_heights.contains(&node.path),
                constrained_by_flex_chain(node, &shrunk_roots, &state_nodes),
            ));
        }
        if !rules.is_empty() {
            let wider = if index == 0 {
                base.viewport.width
            } else {
                states[index - 1].viewport.width
            };
            let smaller = states.get(index + 1).map(|next| next.viewport.width);
            let bounds = band(state.viewport.width, smaller, wider, states.len() == 1);
            css.push_str(&media_rule(bounds.0, bounds.1, &rules));
        }
    }
}

fn shrunk_roots<'a>(
    nodes: &'a [Node],
    base_nodes: &HashMap<&str, &Node>,
    state_nodes: &HashMap<&str, &'a Node>,
) -> HashSet<&'a str> {
    nodes
        .iter()
        .filter(|node| {
            base_nodes.get(node.path.as_str()).is_some_and(|base| {
                shrunk_flex_item(
                    base,
                    node,
                    node.parent
                        .as_deref()
                        .and_then(|parent| state_nodes.get(parent).copied()),
                )
            })
        })
        .map(|node| node.path.as_str())
        .collect()
}

pub(in crate::generate) fn band(
    width: u32,
    smaller: Option<u32>,
    wider: u32,
    sparse: bool,
) -> (Option<u32>, u32) {
    if sparse {
        return (None, wider.saturating_sub(1).max(width));
    }
    (smaller.map(|value| value.saturating_add(1)), width)
}

pub(in crate::generate) fn media_rule(minimum: Option<u32>, maximum: u32, rules: &str) -> String {
    match minimum {
        Some(minimum) => {
            format!("@media(min-width:{minimum}px) and (max-width:{maximum}px){{{rules}}}\n")
        }
        None => format!("@media(max-width:{maximum}px){{{rules}}}\n"),
    }
}
