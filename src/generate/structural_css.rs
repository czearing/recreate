use super::{css::declarations, responsive};
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
            let mut classes = base.clone();
            for node in state.nodes.iter().chain(&state.startup_nodes) {
                if node.tag == "#text"
                    || classes.contains_key(&node.path)
                    || allowed_paths.is_some_and(|paths| !paths.contains(&node.path))
                {
                    continue;
                }
                let class = class_name(node, state, assets);
                append_rule(&class, node, state, assets, css, emitted);
                classes.insert(node.path.clone(), class);
            }
            classes
        })
        .collect()
}

fn class_name(node: &Node, state: &PageState, assets: &BTreeMap<String, String>) -> String {
    let mut signature = responsive::base_declarations(
        node,
        None,
        &state.viewport,
        assets,
        &state.css_rules,
        false,
        false,
    );
    if let Some(before) = &node.before {
        signature.push_str(&before.content);
        signature.push_str(&declarations(&before.style, assets));
    }
    if let Some(after) = &node.after {
        signature.push_str(&after.content);
        signature.push_str(&declarations(&after.style, assets));
    }
    format!("s{}", &hex::encode(Sha256::digest(signature))[..10])
}

fn append_rule(
    class: &str,
    node: &Node,
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
            None,
            &state.viewport,
            assets,
            &state.css_rules,
            false,
            false,
        )
    ));
    if let Some(before) = &node.before {
        css.push_str(&format!(
            ".{class}::before{{content:{};{}}}\n",
            before.content,
            declarations(&before.style, assets)
        ));
    }
    if let Some(after) = &node.after {
        css.push_str(&format!(
            ".{class}::after{{content:{};{}}}\n",
            after.content,
            declarations(&after.style, assets)
        ));
    }
}
