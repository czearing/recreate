use crate::{
    compare::{Report, detail},
    compare_animation, compare_dom,
    model::{Node, PageState},
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "compare_node/attributes.rs"]
mod attributes;
#[path = "compare_node/styles.rs"]
mod styles;

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
    let attributes = attributes::differences(
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
    if !styles::same_pseudos(&expected.pseudos, &actual.pseudos) {
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
    let styles = styles::differences(expected, actual, &animated);
    if !styles.is_empty() {
        report.style_mismatches += 1;
        detail(
            report,
            format!("style {} {}", expected.path, styles.join(",")),
        );
    }
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
