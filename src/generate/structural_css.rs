use super::css_signature::Signature;
use super::responsive;
use crate::model::{Node, PageState};
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
                let rule = rule_parts(node, parent, state, assets);
                let class = class_name(&rule);
                append_rule(&class, &rule, css, emitted);
                classes.insert(node.path.clone(), class);
            }
            classes
        })
        .collect()
}

/// The rule set an element receives: a selector suffix and the declarations that follow it. The
/// empty suffix is the element's own rule; the others are the boxes it generates.
///
/// Built once and used for both the name and the output, so the two cannot describe different
/// elements. Reading the parts twice was also how the same declarations came to be computed
/// twice per element.
fn rule_parts<'a>(
    node: &'a Node,
    parent: Option<&'a Node>,
    state: &'a PageState,
    assets: &BTreeMap<String, String>,
) -> Vec<(&'a str, String)> {
    let mut parts = vec![(
        "",
        responsive::base_declarations(
            node,
            parent,
            &state.viewport,
            assets,
            &state.css_rules,
            false,
        ),
    )];
    for (suffix, pseudo) in super::css_pseudo::slots(node) {
        parts.push((suffix, super::css_pseudo::declarations(pseudo, assets)));
    }
    parts
}

/// Names an element by the rule set it will receive, so two elements share a class exactly when
/// they would be given the same rules.
///
/// The selector suffix is part of what is folded in, which is what keeps an element decorated
/// before its content distinct from one decorated after it. Hashing only the payloads, pasted
/// end to end, let those two produce identical bytes; [`append_rule`] writes once per class, so
/// the second element kept the class, was skipped, and rendered the first one's decoration on
/// the wrong side of its content.
fn class_name(parts: &[(&str, String)]) -> String {
    let mut signature = Signature::new();
    for (suffix, declarations) in parts {
        signature.slot();
        signature.pair(suffix, declarations);
    }
    format!("s{}", &signature.finish()[..10])
}

fn append_rule(
    class: &str,
    parts: &[(&str, String)],
    css: &mut String,
    emitted: &mut HashSet<String>,
) {
    if !emitted.insert(class.to_string()) {
        return;
    }
    for (suffix, declarations) in parts {
        css.push_str(&format!(".{class}{suffix}{{{declarations}}}\n"));
    }
}
