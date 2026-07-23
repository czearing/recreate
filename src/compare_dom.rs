use crate::{
    compare::{Report, detail},
    compare_animation, compare_css_value,
    model::{DomNode, PageState, Rect},
};

pub fn compare(report: &mut Report, expected: &PageState, actual: &PageState) {
    if expected.dom.is_empty() {
        return;
    }
    let expected_style = StyleTable::new(expected);
    let actual_style = StyleTable::new(actual);
    for (path, node) in &expected.dom {
        let Some(candidate) = actual.dom.get(path) else {
            report.structure_mismatches += 1;
            detail(report, format!("dom missing {path}"));
            continue;
        };
        let phase_shifted = compare_animation::phase_shifted_descendant(expected, actual, path);
        let animated = if compare_animation::equivalent_at(expected, actual, path) {
            compare_animation::properties(expected, path)
        } else {
            std::collections::BTreeSet::new()
        };
        compare_structure(report, path, node, candidate);
        compare_geometry(report, path, node, candidate, phase_shifted);
        compare_style(
            report,
            path,
            node,
            candidate,
            &expected_style,
            &actual_style,
            &animated,
        );
    }
}

fn compare_structure(report: &mut Report, path: &str, left: &DomNode, right: &DomNode) {
    if (
        &left.namespace,
        left.node_type,
        &left.tree_scope,
        &left.physical_parent,
        &left.assigned_slot,
        &left.shadow_root_mode,
    ) != (
        &right.namespace,
        right.node_type,
        &right.tree_scope,
        &right.physical_parent,
        &right.assigned_slot,
        &right.shadow_root_mode,
    ) {
        report.structure_mismatches += 1;
        detail(report, format!("dom structure {path}"));
    }
}

fn compare_geometry(
    report: &mut Report,
    path: &str,
    left: &DomNode,
    right: &DomNode,
    phase_shifted: bool,
) {
    let metrics = [
        (left.scroll_left, right.scroll_left),
        (left.scroll_top, right.scroll_top),
        (left.scroll_width, right.scroll_width),
        (left.scroll_height, right.scroll_height),
        (left.client_width, right.client_width),
        (left.client_height, right.client_height),
    ];
    if !phase_shifted
        && (!same_rects(&left.client_rects, &right.client_rects)
            || metrics
                .into_iter()
                .any(|(left, right)| (left - right).abs() > 1.5 + 1.0 / 64.0))
    {
        report.geometry_mismatches += 1;
        detail(report, format!("dom geometry {path}"));
    }
}

fn compare_style(
    report: &mut Report,
    path: &str,
    left: &DomNode,
    right: &DomNode,
    left_table: &StyleTable,
    right_table: &StyleTable,
    animated: &std::collections::BTreeSet<String>,
) {
    let difference = left_table
        .properties
        .iter()
        .enumerate()
        .find(|(index, property)| {
            if property.starts_with("--")
                || (same_rects(&left.client_rects, &right.client_rects)
                    && compare_css_value::layout_property(property))
                || animated.contains(property.as_str())
                || (!animated.is_empty() && compare_css_value::animation_property(property))
                || inactive_rule(property, left, right, left_table, right_table)
            {
                return false;
            }
            !compare_css_value::equivalent(
                left_table.value(left, *index),
                right_table.value_for(right, property),
            )
        })
        .map(|(_, property)| property.as_str());
    if let Some(property) = difference {
        report.style_mismatches += 1;
        detail(report, format!("computed style {path} {property}"));
    }

    fn inactive_rule(
        property: &str,
        left: &DomNode,
        right: &DomNode,
        left_table: &StyleTable,
        right_table: &StyleTable,
    ) -> bool {
        let style = match property {
            "column-rule-width" => "column-rule-style",
            "row-rule-width" => "row-rule-style",
            _ => return false,
        };
        left_table.value_for(left, style) == Some("none")
            && right_table.value_for(right, style) == Some("none")
    }
}

fn same_rects(left: &[Rect], right: &[Rect]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            [
                (left.x, right.x),
                (left.y, right.y),
                (left.width, right.width),
                (left.height, right.height),
            ]
            .into_iter()
            .all(|(left, right)| (left - right).abs() <= 1.5 + 1.0 / 64.0)
        })
}

struct StyleTable<'a> {
    properties: &'a [String],
    dictionary: &'a [String],
}

impl<'a> StyleTable<'a> {
    fn new(state: &'a PageState) -> Self {
        let root = state.dom.get("html");
        Self {
            properties: root.map_or(&[], |node| node.computed_style_properties.as_slice()),
            dictionary: root.map_or(&[], |node| node.computed_style_dictionary.as_slice()),
        }
    }

    fn value(&self, node: &DomNode, index: usize) -> Option<&str> {
        node.computed_style_values
            .get(index)
            .and_then(|value| self.dictionary.get(*value as usize))
            .map(String::as_str)
    }

    fn value_for(&self, node: &DomNode, property: &str) -> Option<&str> {
        self.properties
            .binary_search_by(|candidate| candidate.as_str().cmp(property))
            .ok()
            .and_then(|index| self.value(node, index))
    }
}
