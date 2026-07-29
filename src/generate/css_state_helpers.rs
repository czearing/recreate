use crate::model::{PageState, Specification};
use std::collections::{HashMap, HashSet};

pub fn with_baseline_css(mut state: PageState, baseline: &PageState) -> PageState {
    let mut rules = baseline.css_rules.clone();
    for rule in std::mem::take(&mut state.css_rules) {
        if !rules.contains(&rule) {
            rules.push(rule);
        }
    }
    state.css_rules = rules;
    state
}

pub fn fluid_height_paths(specification: &Specification) -> HashSet<String> {
    let mut heights = HashMap::<String, Vec<f64>>::new();
    let mut authored = HashSet::new();
    for state in &specification.states {
        for node in &state.nodes {
            heights
                .entry(node.path.clone())
                .or_default()
                .push(node.rect.height);
            if super::authored_css::has_property(node, &state.css_rules, "height") {
                authored.insert(node.path.clone());
            }
        }
    }
    heights
        .into_iter()
        .filter(|(path, values)| {
            !authored.contains(path)
                && !specification.states.iter().any(|state| {
                    state
                        .nodes
                        .iter()
                        .find(|node| &node.path == path)
                        .is_some_and(|node| {
                            node.style
                                .get("overflow")
                                .is_some_and(|value| value == "hidden")
                                || node
                                    .style
                                    .get("overflow-y")
                                    .is_some_and(|value| value == "hidden")
                                || node.style.contains_key("-webkit-line-clamp")
                        })
                })
                && values
                    .iter()
                    .skip(1)
                    .any(|value| (value - values[0]).abs() > 1.0)
        })
        .map(|(path, _)| path)
        .collect()
}
