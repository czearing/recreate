use super::super::Frame;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn compare(
    name: &str,
    source: &Frame,
    candidate: &Frame,
    active_styles: &BTreeMap<String, BTreeSet<String>>,
    details: &mut Vec<String>,
) {
    let expected_root = root_origin(source);
    let actual_root = root_origin(candidate);
    if source.snapshot.root_hovered != candidate.snapshot.root_hovered {
        details.push(format!(
            "{name}: hover activation at {}ms source={} candidate={} candidate_hit={:?}",
            source.elapsed_ms,
            source.snapshot.root_hovered,
            candidate.snapshot.root_hovered,
            candidate.snapshot.hit_path
        ));
    }
    let actual: BTreeMap<_, _> = candidate
        .snapshot
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    for expected in &source.snapshot.nodes {
        let Some(node) = actual.get(expected.path.as_str()) else {
            details.push(format!("{name}: missing node {}", expected.path));
            continue;
        };
        let expected_rect = relative_rect(expected.rect, expected_root);
        let actual_rect = relative_rect(node.rect, actual_root);
        let delta = expected_rect
            .iter()
            .zip(actual_rect)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0, f64::max);
        if delta > 1.0 {
            details.push(format!(
                "{name}: geometry {} at {}ms delta={delta:.2}",
                expected.path, source.elapsed_ms
            ));
        }
        let mut compared = BTreeSet::from([
            "opacity",
            "color",
            "backgroundColor",
            "boxShadow",
            "fill",
            "stroke",
            "borderTopColor",
            "borderRightColor",
            "borderBottomColor",
            "borderLeftColor",
        ]);
        compared.extend(
            active_styles
                .get(&expected.path)
                .into_iter()
                .flatten()
                .map(String::as_str),
        );
        for property in compared {
            if expected.style.get(property) != node.style.get(property) {
                details.push(format!(
                    "{name}: style {} {property} at {}ms",
                    expected.path, source.elapsed_ms
                ));
            }
        }
        if expected.text != node.text {
            details.push(format!(
                "{name}: text {} at {}ms source={:?} candidate={:?}",
                expected.path, source.elapsed_ms, expected.text, node.text
            ));
        }
    }
    if source.snapshot.document != candidate.snapshot.document {
        details.push(format!("{name}: document geometry changed"));
    }
    details.truncate(100);
}

fn root_origin(frame: &Frame) -> [f64; 2] {
    frame
        .snapshot
        .nodes
        .iter()
        .find(|node| node.path == ".")
        .map(|node| [node.rect[0], node.rect[1]])
        .unwrap_or_default()
}

fn relative_rect(rect: [f64; 4], origin: [f64; 2]) -> [f64; 4] {
    [rect[0] - origin[0], rect[1] - origin[1], rect[2], rect[3]]
}
