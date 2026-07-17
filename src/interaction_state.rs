use crate::model::PageState;
use std::collections::HashSet;

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

pub fn content_differs(left: &PageState, right: &PageState) -> bool {
    meaningfully_differs(left, right)
        || left
            .nodes
            .iter()
            .zip(&right.nodes)
            .any(|(left, right)| left.text != right.text)
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
