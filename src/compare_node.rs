use crate::{
    compare::{Report, detail},
    compare_animation, compare_css_value, compare_dom,
    model::{Node, PageState},
};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
pub(crate) fn compare(expected: &PageState, actual_state: &PageState) -> Report {
    compare_with_assets(expected, actual_state, &expected.asset_data)
}

#[cfg(test)]
pub(crate) fn compare_with_assets(
    expected: &PageState,
    actual_state: &PageState,
    shared_assets: &BTreeMap<String, String>,
) -> Report {
    compare_with_animation_assets(expected, actual_state, expected, shared_assets)
}

pub(crate) fn compare_with_animation_assets(
    expected: &PageState,
    actual_state: &PageState,
    animation_state: &PageState,
    shared_assets: &BTreeMap<String, String>,
) -> Report {
    let actual: BTreeMap<_, _> = actual_state
        .nodes
        .iter()
        .map(|node| (&node.path, node))
        .collect();
    let expected_paths = expected
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .collect::<BTreeSet<_>>();
    let mut report = empty_report(expected.nodes.len(), actual.len());
    for node in &expected.nodes {
        let Some(candidate) = actual.get(&node.path) else {
            report.missing += 1;
            detail(&mut report, format!("missing {}", node.path));
            continue;
        };
        compare_node(
            &mut report,
            node,
            candidate,
            expected,
            actual_state,
            animation_state,
            shared_assets,
        );
    }
    for path in actual
        .keys()
        .filter(|path| !expected_paths.contains(path.as_str()))
    {
        report.unexpected += 1;
        detail(&mut report, format!("unexpected {path}"));
    }
    compare_dom::compare(&mut report, expected, actual_state);
    report
}

fn compare_node(
    report: &mut Report,
    expected: &Node,
    actual: &Node,
    expected_state: &PageState,
    actual_state: &PageState,
    animation_state: &PageState,
    shared_assets: &BTreeMap<String, String>,
) {
    report.matched += 1;
    if expected.tag != actual.tag || expected.parent != actual.parent {
        report.structure_mismatches += 1;
        detail(report, format!("structure {}", expected.path));
    }
    let attributes = attribute_differences(
        expected,
        actual,
        expected_state,
        actual_state,
        shared_assets,
    );
    if !attributes.is_empty() {
        report.attribute_mismatches += 1;
        detail(
            report,
            format!("attributes {} {}", expected.path, attributes.join(",")),
        );
    }
    if !same_pseudo(expected.before.as_ref(), actual.before.as_ref())
        || !same_pseudo(expected.after.as_ref(), actual.after.as_ref())
    {
        report.pseudo_mismatches += 1;
        detail(report, format!("pseudo {}", expected.path));
    }
    if expected.text != actual.text {
        report.text_mismatches += 1;
        detail(report, format!("text {}", expected.path));
    }
    let phase_shifted =
        compare_animation::phase_shifted_descendant(animation_state, actual_state, &expected.path);
    if !same_rect(expected, actual) && !phase_shifted {
        report.geometry_mismatches += 1;
        detail(
            report,
            format!(
                "rect {} expected={:?} actual={:?}",
                expected.path, expected.rect, actual.rect
            ),
        );
    }
    let animated =
        if compare_animation::equivalent_at(animation_state, actual_state, &expected.path) {
            compare_animation::properties(animation_state, &expected.path)
        } else if expected
            .style
            .get("animation")
            .is_some_and(|value| value != "none")
            && compare_animation::equivalent_anywhere(animation_state, actual_state, &expected.path)
        {
            compare_animation::properties(actual_state, &expected.path)
        } else {
            BTreeSet::new()
        };
    let styles = style_differences(expected, actual, &animated);
    if !styles.is_empty() {
        report.style_mismatches += 1;
        detail(
            report,
            format!("style {} {}", expected.path, styles.join(",")),
        );
    }
}

fn attribute_differences(
    expected: &Node,
    actual: &Node,
    expected_state: &PageState,
    actual_state: &PageState,
    shared_assets: &BTreeMap<String, String>,
) -> Vec<String> {
    let keys = expected
        .attributes
        .keys()
        .chain(actual.attributes.keys())
        .filter(|key| comparable_attribute(key))
        .collect::<BTreeSet<_>>();
    keys.into_iter()
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

fn comparable_attribute(key: &str) -> bool {
    !matches!(key, "class" | "style") && !key.starts_with("data-recreate-")
}

pub(crate) fn same_rect(left: &Node, right: &Node) -> bool {
    const TOLERANCE: f64 = 1.5 + 1.0 / 64.0;
    [
        (left.rect.x, right.rect.x),
        (left.rect.y, right.rect.y),
        (left.rect.width, right.rect.width),
        (left.rect.height, right.rect.height),
    ]
    .into_iter()
    .all(|(left, right)| (left - right).abs() <= TOLERANCE)
}

fn style_differences(left: &Node, right: &Node, animated: &BTreeSet<String>) -> Vec<String> {
    let same_geometry = same_rect(left, right);
    left.style
        .keys()
        .chain(right.style.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|key| {
            !(animated.contains(*key)
                || compare_css_value::equivalent(
                    left.style.get(*key).map(String::as_str),
                    right.style.get(*key).map(String::as_str),
                )
                || (same_geometry && compare_css_value::layout_property(key))
                || (!animated.is_empty() && compare_css_value::animation_property(key)))
        })
        .map(|key| {
            format!(
                "{key}={:?}/{:?}",
                left.style.get(key).map(String::as_str),
                right.style.get(key).map(String::as_str)
            )
        })
        .collect()
}

fn same_pseudo(left: Option<&crate::model::Pseudo>, right: Option<&crate::model::Pseudo>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.content == right.content
                && style_differences_for(&left.style, &right.style).is_empty()
        }
        _ => false,
    }
}

fn style_differences_for(left: &crate::model::Styles, right: &crate::model::Styles) -> Vec<String> {
    left.keys()
        .chain(right.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|key| {
            !compare_css_value::equivalent(
                left.get(*key).map(String::as_str),
                right.get(*key).map(String::as_str),
            )
        })
        .cloned()
        .collect()
}

fn empty_report(expected: usize, actual: usize) -> Report {
    Report {
        passed: true,
        expected,
        actual,
        matched: 0,
        missing: 0,
        unexpected: 0,
        structure_mismatches: 0,
        attribute_mismatches: 0,
        pseudo_mismatches: 0,
        text_mismatches: 0,
        geometry_mismatches: 0,
        style_mismatches: 0,
        details: Vec::new(),
    }
}
