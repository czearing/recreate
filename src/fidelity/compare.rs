use super::{Frame, Trace};
use std::collections::{BTreeMap, BTreeSet};

#[path = "compare_frame.rs"]
mod compare_frame;

pub(super) fn traces(source: &Trace, candidate: &Trace) -> Vec<String> {
    let mut details = Vec::new();
    phase("hover", &source.hover, &candidate.hover, &mut details);
    phase("leave", &source.leave, &candidate.leave, &mut details);
    details
}

fn phase(name: &str, source: &[Frame], candidate: &[Frame], details: &mut Vec<String>) {
    let source_motion = motion(source);
    let candidate_motion = motion(candidate);
    let mut active_styles = style_changes(source);
    for (path, properties) in style_changes(candidate) {
        active_styles.entry(path).or_default().extend(properties);
    }
    for (path, properties) in &source_motion {
        match candidate_motion.get(path) {
            None => details.push(format!("{name}: missing animated target {path}")),
            Some(actual) if actual != properties => details.push(format!(
                "{name}: properties {path}: source={properties:?} candidate={actual:?}"
            )),
            _ => {}
        }
    }
    if let (Some(expected), Some(actual)) = (source.first(), candidate.first()) {
        compare_frame::compare(name, expected, actual, &active_styles, details);
    }
    if let (Some(expected), Some(actual)) = (source.last(), candidate.last()) {
        compare_frame::compare(name, expected, actual, &active_styles, details);
    }
    compare_geometry_ranges(name, source, candidate, details);
}

fn motion(frames: &[Frame]) -> BTreeMap<String, BTreeSet<String>> {
    let mut values = BTreeMap::<String, BTreeSet<String>>::new();
    for animation in frames.iter().flat_map(|frame| &frame.snapshot.animations) {
        for property in &animation.properties {
            values
                .entry(animation.target.clone())
                .or_default()
                .insert(format!(
                    "{property}|{:.2}|{:.2}|{}|{}",
                    animation.duration,
                    animation.delay,
                    animation.easing,
                    animation.pseudo.as_deref().unwrap_or_default()
                ));
        }
    }
    values
}

fn style_changes(frames: &[Frame]) -> BTreeMap<String, BTreeSet<String>> {
    let Some((first, last)) = frames.first().zip(frames.last()) else {
        return BTreeMap::new();
    };
    let last: BTreeMap<_, _> = last
        .snapshot
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    let mut changes = BTreeMap::<String, BTreeSet<String>>::new();
    for node in &first.snapshot.nodes {
        let Some(final_node) = last.get(node.path.as_str()) else {
            continue;
        };
        for property in [
            "opacity",
            "color",
            "backgroundColor",
            "boxShadow",
            "fill",
            "stroke",
        ] {
            if node.style.get(property) != final_node.style.get(property) {
                changes
                    .entry(node.path.clone())
                    .or_default()
                    .insert(property.into());
            }
        }
    }
    changes
}

fn compare_geometry_ranges(
    name: &str,
    source: &[Frame],
    candidate: &[Frame],
    details: &mut Vec<String>,
) {
    let expected = geometry_ranges(source);
    let actual = geometry_ranges(candidate);
    for (path, expected_range) in expected {
        let Some(actual_range) = actual.get(&path) else {
            continue;
        };
        let delta = expected_range
            .iter()
            .zip(actual_range)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0, f64::max);
        if delta > 1.0 {
            details.push(format!(
                "{name}: geometry trajectory {path} delta={delta:.2}"
            ));
        }
    }
}

fn geometry_ranges(frames: &[Frame]) -> BTreeMap<String, [f64; 8]> {
    let mut ranges = BTreeMap::new();
    let origin = frames
        .first()
        .and_then(|frame| frame.snapshot.nodes.iter().find(|node| node.path == "."))
        .map(|node| [node.rect[0], node.rect[1]])
        .unwrap_or_default();
    for node in frames.iter().flat_map(|frame| &frame.snapshot.nodes) {
        let range = ranges
            .entry(node.path.clone())
            .or_insert([f64::INFINITY; 8]);
        let rect = [
            node.rect[0] - origin[0],
            node.rect[1] - origin[1],
            node.rect[2],
            node.rect[3],
        ];
        for (index, value) in rect.iter().enumerate() {
            range[index] = range[index].min(*value);
            let maximum = index + 4;
            range[maximum] = if range[maximum].is_infinite() {
                *value
            } else {
                range[maximum].max(*value)
            };
        }
    }
    ranges
}
