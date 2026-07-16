use crate::{
    compare::{Report, detail},
    model::{Node, PageState},
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn compare(expected: &PageState, actual: &PageState) -> Report {
    let actual: BTreeMap<_, _> = actual.nodes.iter().map(|node| (&node.path, node)).collect();
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
        compare_node(&mut report, node, candidate);
    }
    for path in actual
        .keys()
        .filter(|path| !expected_paths.contains(path.as_str()))
    {
        report.unexpected += 1;
        detail(&mut report, format!("unexpected {path}"));
    }
    report
}

fn compare_node(report: &mut Report, expected: &Node, actual: &Node) {
    report.matched += 1;
    if expected.tag != actual.tag || expected.parent != actual.parent {
        report.structure_mismatches += 1;
        detail(report, format!("structure {}", expected.path));
    }
    let attributes = attribute_differences(expected, actual);
    if !attributes.is_empty() {
        report.attribute_mismatches += 1;
        detail(
            report,
            format!("attributes {} {}", expected.path, attributes.join(",")),
        );
    }
    if pseudo_content(expected) != pseudo_content(actual) {
        report.pseudo_mismatches += 1;
        detail(report, format!("pseudo {}", expected.path));
    }
    if expected.text != actual.text {
        report.text_mismatches += 1;
        detail(report, format!("text {}", expected.path));
    }
    if !same_rect(expected, actual) {
        report.geometry_mismatches += 1;
        detail(
            report,
            format!(
                "rect {} expected={:?} actual={:?}",
                expected.path, expected.rect, actual.rect
            ),
        );
    }
    if !same_style(expected, actual) {
        report.style_mismatches += 1;
        detail(report, format!("style {}", expected.path));
    }
}

fn attribute_differences(expected: &Node, actual: &Node) -> Vec<String> {
    expected
        .attributes
        .iter()
        .filter(|(key, value)| {
            semantic_attribute(key) && actual.attributes.get(*key) != Some(*value)
        })
        .map(|(key, value)| {
            format!(
                "{key}={value:?}/{:?}",
                actual.attributes.get(key).map(String::as_str)
            )
        })
        .collect()
}

fn semantic_attribute(key: &str) -> bool {
    key.starts_with("aria-")
        || matches!(
            key,
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
                | "for"
        )
}

fn pseudo_content(node: &Node) -> (Option<&str>, Option<&str>) {
    (
        node.before.as_ref().map(|pseudo| pseudo.content.as_str()),
        node.after.as_ref().map(|pseudo| pseudo.content.as_str()),
    )
}

pub(crate) fn same_rect(left: &Node, right: &Node) -> bool {
    if left.rect.width == 0.0 || left.rect.height == 0.0 {
        return true;
    }
    [
        (left.rect.x, right.rect.x),
        (left.rect.y, right.rect.y),
        (left.rect.width, right.rect.width),
        (left.rect.height, right.rect.height),
    ]
    .into_iter()
    .all(|(left, right)| (left - right).abs() <= 1.5)
}

fn same_style(left: &Node, right: &Node) -> bool {
    [
        "color",
        "background-color",
        "font-family",
        "font-size",
        "font-weight",
        "border-radius",
        "display",
        "position",
    ]
    .into_iter()
    .all(|key| left.style.get(key) == right.style.get(key))
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
