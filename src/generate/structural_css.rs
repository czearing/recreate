use super::responsive;
use crate::model::{Node, PageState};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

pub fn class_maps(
    states: &[PageState],
    base: &BTreeMap<String, String>,
    assets: &BTreeMap<String, String>,
    css: &mut String,
    emitted: &mut HashSet<String>,
    allowed_paths: Option<&HashSet<String>>,
) -> Vec<BTreeMap<String, String>> {
    states
        .iter()
        .map(|state| {
            let nodes: BTreeMap<_, _> = state
                .nodes
                .iter()
                .chain(&state.startup_nodes)
                .map(|node| (node.path.as_str(), node))
                .collect();
            let mut classes = base.clone();
            for node in state.nodes.iter().chain(&state.startup_nodes) {
                if node.tag == "#text"
                    || classes.contains_key(&node.path)
                    || allowed_paths.is_some_and(|paths| !paths.contains(&node.path))
                {
                    continue;
                }
                let parent = node
                    .parent
                    .as_deref()
                    .and_then(|parent| nodes.get(parent).copied());
                let class = class_name(node, parent, state, assets);
                append_rule(&class, node, parent, state, assets, css, emitted);
                classes.insert(node.path.clone(), class);
            }
            classes
        })
        .collect()
}

fn class_name(
    node: &Node,
    parent: Option<&Node>,
    state: &PageState,
    assets: &BTreeMap<String, String>,
) -> String {
    let mut signature = responsive::base_declarations(
        node,
        parent,
        &state.viewport,
        assets,
        &state.css_rules,
        false,
    );
    if let Some(before) = &node.before {
        signature.push_str(&before.content);
        signature.push_str(&responsive::output_declarations(&before.style, assets));
    }
    if let Some(after) = &node.after {
        signature.push_str(&after.content);
        signature.push_str(&responsive::output_declarations(&after.style, assets));
    }
    format!("s{}", &hex::encode(Sha256::digest(signature))[..10])
}

fn append_rule(
    class: &str,
    node: &Node,
    parent: Option<&Node>,
    state: &PageState,
    assets: &BTreeMap<String, String>,
    css: &mut String,
    emitted: &mut HashSet<String>,
) {
    if !emitted.insert(class.to_string()) {
        return;
    }
    css.push_str(&format!(
        ".{class}{{{}}}\n",
        responsive::base_declarations(
            node,
            parent,
            &state.viewport,
            assets,
            &state.css_rules,
            false,
        )
    ));
    if let Some(before) = &node.before {
        css.push_str(&format!(
            ".{class}::before{{content:{};{}}}\n",
            before.content,
            responsive::output_declarations(&before.style, assets)
        ));
    }
    if let Some(after) = &node.after {
        css.push_str(&format!(
            ".{class}::after{{content:{};{}}}\n",
            after.content,
            responsive::output_declarations(&after.style, assets)
        ));
    }
}
