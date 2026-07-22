use crate::model::PageState;
use std::collections::{HashMap, HashSet};

#[cfg(test)]
pub fn differs(left: &PageState, right: &PageState) -> bool {
    left.nodes != right.nodes
}

pub fn meaningfully_differs(left: &PageState, right: &PageState) -> bool {
    left.nodes.len() != right.nodes.len()
        || left.nodes.iter().zip(&right.nodes).any(|(left, right)| {
            left.path != right.path
                || left.tag != right.tag
                || semantic_attributes(left) != semantic_attributes(right)
        })
}

#[cfg(test)]
pub fn content_differs(left: &PageState, right: &PageState) -> bool {
    meaningfully_differs(left, right)
        || left
            .nodes
            .iter()
            .zip(&right.nodes)
            .any(|(left, right)| left.text != right.text)
}

pub fn selected_differs(left: &PageState, right: &PageState) -> bool {
    let baseline: HashMap<_, _> = right
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    left.nodes.iter().any(|node| {
        baseline.get(node.path.as_str()).is_none_or(|baseline| {
            node.tag != baseline.tag || semantic_attributes(node) != semantic_attributes(baseline)
        })
    })
}

pub fn surface_differs(left: &PageState, right: &PageState, trigger: &str, label: &str) -> bool {
    let baseline: HashMap<_, _> = right
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    let added_portals = left
        .nodes
        .iter()
        .filter(|node| {
            node.attributes.contains_key("data-portal-node")
                && !baseline.contains_key(node.path.as_str())
        })
        .map(|node| node.path.as_str())
        .collect::<Vec<_>>();
    left.nodes.iter().any(|node| {
        !is_trigger_node(node.path.as_str(), trigger)
            && visible(node)
            && (overlay(node, label)
                || added_portals.iter().any(|root| {
                    node.path
                        .strip_prefix(root)
                        .is_some_and(|suffix| suffix.starts_with('>'))
                }))
            && baseline
                .get(node.path.as_str())
                .is_none_or(|node| !visible(node))
    })
}

fn is_trigger_node(path: &str, trigger: &str) -> bool {
    path == trigger
        || path
            .strip_prefix(trigger)
            .is_some_and(|suffix| suffix.starts_with('>'))
}

fn visible(node: &crate::model::Node) -> bool {
    node.rect.width > 0.0
        && node.rect.height > 0.0
        && node
            .style
            .get("display")
            .is_none_or(|value| value != "none")
        && node
            .style
            .get("visibility")
            .is_none_or(|value| value != "hidden")
        && node
            .style
            .get("opacity")
            .and_then(|value| value.parse::<f64>().ok())
            .is_none_or(|value| value > 0.01)
}

fn overlay(node: &crate::model::Node, label: &str) -> bool {
    node.tag != "#text"
        && (node.attributes.contains_key("data-portal-node")
            || node.attributes.get("role").is_some_and(|role| {
                matches!(
                    role.as_str(),
                    "dialog" | "listbox" | "menu" | "menuitem" | "option"
                )
            })
            || node
                .style
                .get("position")
                .is_some_and(|value| value == "fixed")
            || (node
                .style
                .get("position")
                .is_some_and(|value| value == "absolute")
                && !node.text.trim().is_empty()
                && !node.text.trim().eq_ignore_ascii_case(label)))
}

fn semantic_attributes(node: &crate::model::Node) -> Vec<(&str, &str)> {
    node.attributes
        .iter()
        .filter(|(key, _)| {
            key.starts_with("aria-")
                || matches!(
                    key.as_str(),
                    "role"
                        | "href"
                        | "target"
                        | "type"
                        | "disabled"
                        | "tabindex"
                        | "name"
                        | "value"
                        | "checked"
                        | "selected"
                )
        })
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect()
}

pub fn compact(state: &mut PageState, baseline: &PageState, settled: bool) {
    state.attribute_sequences.clear();
    if settled {
        state.animations.clear();
    } else {
        state.animations.retain(|animation| {
            !animation.keyframes.iter().any(|frame| {
                ["x", "y", "width", "height"]
                    .into_iter()
                    .all(|key| frame.get(key).is_some())
            })
        });
    }
    let css: HashSet<_> = baseline.css_rules.iter().map(String::as_str).collect();
    state.css_rules.retain(|rule| !css.contains(rule.as_str()));
    let assets: HashSet<_> = baseline.asset_urls.iter().map(String::as_str).collect();
    state
        .asset_urls
        .retain(|url| !assets.contains(url.as_str()));
    state
        .asset_data
        .retain(|url, data| baseline.asset_data.get(url) != Some(data));
    state
        .state_styles
        .retain(|style| !baseline.state_styles.contains(style));
}

#[cfg(test)]
#[path = "interaction_state_tests.rs"]
mod tests;
