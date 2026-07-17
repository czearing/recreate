use super::{names, structural_tree};
use crate::model::{Node, Specification};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone)]
pub struct Component {
    pub name: String,
    pub roots: Vec<String>,
}

pub struct Components {
    pub items: Vec<Component>,
    pub by_root: BTreeMap<String, usize>,
    pub children: BTreeMap<String, Vec<String>>,
    pub classes: BTreeMap<String, String>,
    pub nodes: BTreeMap<String, Node>,
}

pub fn components(specification: &Specification, classes: &BTreeMap<String, String>) -> Components {
    let Some(state) = specification.states.first() else {
        return Components {
            items: Vec::new(),
            by_root: BTreeMap::new(),
            children: BTreeMap::new(),
            classes: BTreeMap::new(),
            nodes: BTreeMap::new(),
        };
    };
    let children = structural_tree::children(&state.nodes);
    let nodes: BTreeMap<_, _> = state
        .nodes
        .iter()
        .map(|node| (node.path.clone(), node))
        .collect();
    let mut memo = HashMap::new();
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for node in &state.nodes {
        let fingerprint = fingerprint(&node.path, &nodes, &children, classes, &mut memo);
        groups
            .entry(fingerprint)
            .or_default()
            .push(node.path.clone());
    }
    let mut sizes = HashMap::new();
    let mut candidates: Vec<(Vec<String>, usize)> = groups
        .into_values()
        .filter(|roots| roots.len() >= 2)
        .filter_map(|roots| {
            let size = subtree_size(&roots[0], &children, &mut sizes);
            (2..=120).contains(&size).then_some((roots, size))
        })
        .collect();
    candidates.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
    candidates.truncate(80);
    let mut names = HashMap::new();
    let items: Vec<Component> = candidates
        .into_iter()
        .enumerate()
        .map(|(index, (roots, _))| {
            let node = nodes.get(&roots[0]).copied();
            let base = node
                .map(|node| names::for_node(node, index))
                .unwrap_or_else(|| format!("Component{}", index + 1));
            let count = names.entry(base.clone()).or_insert(0_usize);
            *count += 1;
            let name = if *count == 1 {
                base
            } else {
                format!("{base}{count}")
            };
            Component { name, roots }
        })
        .collect();
    let by_root = items
        .iter()
        .enumerate()
        .flat_map(|(index, component)| {
            component
                .roots
                .iter()
                .cloned()
                .map(move |root| (root, index))
        })
        .collect();
    Components {
        items,
        by_root,
        children,
        classes: classes.clone(),
        nodes: state
            .nodes
            .iter()
            .cloned()
            .map(|node| (node.path.clone(), node))
            .collect(),
    }
}

fn fingerprint(
    path: &str,
    nodes: &BTreeMap<String, &Node>,
    children: &BTreeMap<String, Vec<String>>,
    classes: &BTreeMap<String, String>,
    memo: &mut HashMap<String, String>,
) -> String {
    if let Some(value) = memo.get(path) {
        return value.clone();
    }
    let Some(node) = nodes.get(path) else {
        return String::new();
    };
    let child_values = children
        .get(path)
        .into_iter()
        .flatten()
        .map(|child| fingerprint(child, nodes, children, classes, memo))
        .collect::<Vec<_>>()
        .join(",");
    let attributes = node
        .attributes
        .keys()
        .filter(|key| !dynamic_attribute(key))
        .cloned()
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "{}|{}|{}|{}",
        node.tag,
        classes.get(path).map(String::as_str).unwrap_or_default(),
        attributes,
        child_values
    );
    let value = hex::encode(Sha256::digest(source.as_bytes()));
    memo.insert(path.to_string(), value.clone());
    value
}

pub fn dynamic_attribute(name: &str) -> bool {
    !matches!(name, "class" | "style") && !name.starts_with("on")
}

fn subtree_size(
    path: &str,
    children: &BTreeMap<String, Vec<String>>,
    memo: &mut HashMap<String, usize>,
) -> usize {
    if let Some(size) = memo.get(path) {
        return *size;
    }
    let size = 1 + children
        .get(path)
        .into_iter()
        .flatten()
        .map(|child| subtree_size(child, children, memo))
        .sum::<usize>();
    memo.insert(path.to_string(), size);
    size
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;
