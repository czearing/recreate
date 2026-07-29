use crate::model::{Node, PageState};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn differences(
    expected: &Node,
    actual: &Node,
    expected_state: &PageState,
    actual_state: &PageState,
    shared_assets: &BTreeMap<String, String>,
) -> Vec<String> {
    expected
        .attributes
        .keys()
        .chain(actual.attributes.keys())
        .filter(|key| comparable(key))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|key| {
            let left = expected.attributes.get(*key);
            let right = actual.attributes.get(*key);
            left != right
                && !resource_equivalent(
                    key,
                    left,
                    right,
                    expected_state,
                    actual_state,
                    shared_assets,
                )
        })
        .map(|key| {
            format!(
                "{key}={:?}/{:?}",
                expected.attributes.get(key).map(String::as_str),
                actual.attributes.get(key).map(String::as_str)
            )
        })
        .collect()
}

fn resource_equivalent(
    attribute: &str,
    left: Option<&String>,
    right: Option<&String>,
    left_state: &PageState,
    right_state: &PageState,
    shared_assets: &BTreeMap<String, String>,
) -> bool {
    if !matches!(attribute, "src" | "poster") {
        return false;
    }
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };
    asset_data(left_state, left)
        .or_else(|| asset_data_map(shared_assets, left))
        .zip(asset_data(right_state, right))
        .is_some_and(|(left, right)| left == right)
}

fn asset_data_map<'a>(assets: &'a BTreeMap<String, String>, url: &str) -> Option<&'a str> {
    assets
        .get(url)
        .or_else(|| {
            assets
                .iter()
                .find(|(candidate, _)| candidate.ends_with(url))
                .map(|(_, data)| data)
        })
        .map(String::as_str)
}

fn asset_data<'a>(state: &'a PageState, url: &str) -> Option<&'a str> {
    state
        .asset_data
        .get(url)
        .or_else(|| {
            state
                .asset_data
                .iter()
                .find(|(candidate, _)| candidate.ends_with(url))
                .map(|(_, data)| data)
        })
        .map(String::as_str)
}

fn comparable(key: &str) -> bool {
    !matches!(key, "class" | "style") && !key.starts_with("data-recreate-")
}
