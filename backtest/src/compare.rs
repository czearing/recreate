use crate::{
    digest,
    model::{
        Artifact, Finding, NodeSnapshot, Report, Snapshot, StateEvidence, Status, Viewport,
    },
};
use std::collections::{BTreeMap, BTreeSet};

const STYLE_ORDER: &[&str] = &[
    "display",
    "visibility",
    "position",
    "font-family",
    "font-size",
    "font-weight",
    "line-height",
    "letter-spacing",
    "color",
    "background-color",
    "border-color",
    "border-radius",
    "box-shadow",
    "opacity",
    "transform",
    "overflow",
];

pub fn artifact(expected: &Artifact, actual: &[StateEvidence], elapsed_ms: u128) -> Report {
    let mut findings = BTreeMap::<String, Finding>::new();
    let mut coverage = Vec::new();
    for source in &expected.states {
        let Some(candidate) = actual.iter().find(|value| {
            value.viewport.width == source.viewport.width
                && value.viewport.height == source.viewport.height
        }) else {
            coverage.push(format!(
                "missing viewport {}x{}",
                source.viewport.width, source.viewport.height
            ));
            continue;
        };
        compare_snapshot(
            &source.viewport,
            "base",
            "base",
            &source.baseline,
            &candidate.baseline,
            &mut findings,
        );
        for scenario in &source.scenarios {
            let Some(actual_scenario) = candidate
                .scenarios
                .iter()
                .find(|value| value.id == scenario.id)
            else {
                coverage.push(format!("missing scenario {}", scenario.id));
                continue;
            };
            for checkpoint in &scenario.checkpoints {
                let Some(actual_checkpoint) = actual_scenario
                    .checkpoints
                    .iter()
                    .find(|value| value.virtual_ms == checkpoint.virtual_ms)
                else {
                    coverage.push(format!(
                        "missing checkpoint {}@{}",
                        scenario.id, checkpoint.virtual_ms
                    ));
                    continue;
                };
                compare_snapshot(
                    &source.viewport,
                    &format!("{}ms", checkpoint.virtual_ms),
                    &format!("{}:{}", scenario.action.kind.as_str(), scenario.action.target),
                    &checkpoint.snapshot,
                    &actual_checkpoint.snapshot,
                    &mut findings,
                );
            }
        }
    }
    let findings = findings.into_values().collect::<Vec<_>>();
    let status = if !coverage.is_empty() {
        Status::Inconclusive
    } else if findings.is_empty() {
        Status::Pass
    } else {
        Status::Fail
    };
    Report {
        status,
        elapsed_ms,
        findings,
        coverage,
    }
}

fn compare_snapshot(
    viewport: &Viewport,
    checkpoint: &str,
    action: &str,
    source: &Snapshot,
    candidate: &Snapshot,
    findings: &mut BTreeMap<String, Finding>,
) {
    let expected: BTreeMap<_, _> = source.nodes.iter().map(|node| (&node.id, node)).collect();
    let actual: BTreeMap<_, _> = candidate
        .nodes
        .iter()
        .map(|node| (&node.id, node))
        .collect();
    let mut changed_roots = BTreeSet::new();
    for (id, node) in &expected {
        if changed_ancestor(node, &expected, &changed_roots) {
            continue;
        }
        let Some(other) = actual.get(id) else {
            insert(
                findings,
                finding(
                    viewport.width,
                    checkpoint,
                    action,
                    id,
                    "node",
                    "present",
                    "missing",
                    None,
                ),
            );
            changed_roots.insert((*id).clone());
            continue;
        };
        if let Some(value) = first_node_difference(node, other) {
            insert(
                findings,
                finding(
                    viewport.width,
                    checkpoint,
                    action,
                    id,
                    value.0,
                    &value.1,
                    &value.2,
                    value.3,
                ),
            );
            changed_roots.insert((*id).clone());
        }
    }
    for id in actual.keys().filter(|id| !expected.contains_key(*id)) {
        let node = actual[id];
        if changed_ancestor(node, &actual, &changed_roots) {
            continue;
        }
        insert(
            findings,
            finding(
                viewport.width,
                checkpoint,
                action,
                id,
                "node",
                "missing",
                "present",
                None,
            ),
        );
        changed_roots.insert((*id).clone());
    }
    compare_animations(viewport, checkpoint, action, source, candidate, findings);
    if source.active != candidate.active {
        insert(
            findings,
            finding(
                viewport.width,
                checkpoint,
                action,
                "active-element",
                "focus",
                source.active.as_deref().unwrap_or("none"),
                candidate.active.as_deref().unwrap_or("none"),
                None,
            ),
        );
    }
    if source.console_errors != candidate.console_errors {
        insert(
            findings,
            finding(
                viewport.width,
                checkpoint,
                "load",
                "window",
                "console-errors",
                &source.console_errors.len().to_string(),
                &candidate.console_errors.len().to_string(),
                count_delta(source.console_errors.len(), candidate.console_errors.len()),
            ),
        );
    }
    if source.unexpected_requests != candidate.unexpected_requests {
        insert(
            findings,
            finding(
                viewport.width,
                checkpoint,
                "load",
                "network",
                "unexpected-requests",
                &source.unexpected_requests.len().to_string(),
                &candidate.unexpected_requests.len().to_string(),
                count_delta(
                    source.unexpected_requests.len(),
                    candidate.unexpected_requests.len(),
                ),
            ),
        );
    }
}

fn first_node_difference(
    source: &NodeSnapshot,
    candidate: &NodeSnapshot,
) -> Option<(&'static str, String, String, Option<String>)> {
    if source.tag != candidate.tag || source.parent != candidate.parent {
        return Some((
            "structure",
            format!("{}@{}", source.tag, source.parent.as_deref().unwrap_or("-")),
            format!(
                "{}@{}",
                candidate.tag,
                candidate.parent.as_deref().unwrap_or("-")
            ),
            None,
        ));
    }
    if source.text != candidate.text {
        return Some((
            "text",
            source.text.clone(),
            candidate.text.clone(),
            None,
        ));
    }
    if source.role != candidate.role {
        return Some((
            "role",
            source.role.clone(),
            candidate.role.clone(),
            None,
        ));
    }
    if source.name != candidate.name {
        return Some((
            "name",
            source.name.clone(),
            candidate.name.clone(),
            None,
        ));
    }
    let source_visible = source.rect[2] > 0.0
        && source.rect[3] > 0.0
        && source.style.get("display").is_none_or(|value| value != "none")
        && source
            .style
            .get("visibility")
            .is_none_or(|value| value != "hidden");
    let candidate_visible = candidate.rect[2] > 0.0
        && candidate.rect[3] > 0.0
        && candidate
            .style
            .get("display")
            .is_none_or(|value| value != "none")
        && candidate
            .style
            .get("visibility")
            .is_none_or(|value| value != "hidden");
    if source_visible != candidate_visible {
        return Some((
            "visibility",
            if source_visible { "visible" } else { "hidden" }.into(),
            if candidate_visible { "visible" } else { "hidden" }.into(),
            None,
        ));
    }
    for (index, property) in ["x", "y", "width", "height"].into_iter().enumerate() {
        if (source.rect[index] - candidate.rect[index]).abs() > 0.25 {
            let left = source.rect[index].round();
            let right = candidate.rect[index].round();
            let delta = right - left;
            return Some((
                property,
                format_number(left),
                format_number(right),
                Some(format!("{delta:+.0}px")),
            ));
        }
    }
    for property in STYLE_ORDER {
        let left = source.style.get(*property).map(String::as_str).unwrap_or("");
        let right = candidate
            .style
            .get(*property)
            .map(String::as_str)
            .unwrap_or("");
        if left != right {
            let property = if *property == "background-color" {
                "background"
            } else {
                property
            };
            return Some((
                property,
                normalize_style_value(left),
                normalize_style_value(right),
                None,
            ));
        }
    }
    for (property, left) in &source.attributes {
        let right = candidate
            .attributes
            .get(property)
            .map(String::as_str)
            .unwrap_or("");
        if left != right {
            return Some(("attribute", format!("{property}={left}"), format!("{property}={right}"), None));
        }
    }
    None
}

fn compare_animations(
    viewport: &Viewport,
    checkpoint: &str,
    action: &str,
    source: &Snapshot,
    candidate: &Snapshot,
    findings: &mut BTreeMap<String, Finding>,
) {
    let expected: BTreeMap<_, _> = source
        .animations
        .iter()
        .map(|value| (&value.target, value))
        .collect();
    let actual: BTreeMap<_, _> = candidate
        .animations
        .iter()
        .map(|value| (&value.target, value))
        .collect();
    for (target, animation) in expected {
        let Some(other) = actual.get(target) else {
            insert(
                findings,
                finding(
                    viewport.width,
                    checkpoint,
                    action,
                    target,
                    "animation",
                    "present",
                    "missing",
                    None,
                ),
            );
            continue;
        };
        let values = [
            (
                "duration",
                format!("{:.0}ms", animation.duration),
                format!("{:.0}ms", other.duration),
            ),
            ("delay", animation.delay.to_string(), other.delay.to_string()),
            ("easing", animation.easing.clone(), other.easing.clone()),
            ("direction", animation.direction.clone(), other.direction.clone()),
            ("keyframes", animation.keyframes.clone(), other.keyframes.clone()),
        ];
        if let Some((property, left, right)) =
            values.into_iter().find(|(_, left, right)| left != right)
        {
            insert(
                findings,
                finding(
                    viewport.width,
                    checkpoint,
                    &format!("animation:{target}"),
                    target,
                    property,
                    &left,
                    &right,
                    if property == "duration" {
                        Some(format!(
                            "{:+.0}ms",
                            other.duration - animation.duration
                        ))
                    } else {
                        None
                    },
                ),
            );
        }
    }
}

fn finding(
    viewport: u32,
    checkpoint: &str,
    action: &str,
    target: &str,
    property: &str,
    source: &str,
    candidate: &str,
    delta: Option<String>,
) -> Finding {
    let category = category(property);
    let key = format!("{viewport}|{target}|{property}|{category}");
    Finding {
        id: digest::bytes(key.as_bytes())[..12].into(),
        key,
        category: category.into(),
        viewport,
        checkpoint: checkpoint.into(),
        action: action.into(),
        target: target.into(),
        property: property.into(),
        source: source.into(),
        candidate: candidate.into(),
        delta,
        effects: Vec::new(),
    }
}

fn insert(findings: &mut BTreeMap<String, Finding>, value: Finding) {
    findings.entry(value.key.clone()).or_insert(value);
}

fn changed_ancestor(
    node: &NodeSnapshot,
    nodes: &BTreeMap<&String, &NodeSnapshot>,
    changed: &BTreeSet<String>,
) -> bool {
    let mut parent = node.parent.as_ref();
    while let Some(id) = parent {
        if changed.contains(id) {
            return true;
        }
        parent = nodes.get(id).and_then(|value| value.parent.as_ref());
    }
    false
}

fn category(property: &str) -> &'static str {
    match property {
        "node" | "structure" => "structure",
        "text" | "attribute" => "content",
        "x" | "y" | "width" | "height" => "geometry",
        "role" | "name" => "accessibility",
        "duration" | "delay" | "easing" | "direction" | "keyframes" | "animation" => {
            "animation"
        }
        "console-errors" | "unexpected-requests" => "runtime",
        _ => "style",
    }

    fn normalize_style_value(value: &str) -> String {
        let value = value.trim();
        if let Some(inner) = value
            .strip_prefix("rgb(")
            .and_then(|value| value.strip_suffix(')'))
        {
            let channels = inner
                .split(',')
                .map(|value| value.trim().parse::<u8>())
                .collect::<Result<Vec<_>, _>>();
            if let Ok(channels) = channels
                && channels.len() == 3
            {
                return format!("#{:02x}{:02x}{:02x}", channels[0], channels[1], channels[2]);
            }
        }
        value.into()
    }

    fn count_delta(source: usize, candidate: usize) -> Option<String> {
        let delta = candidate as isize - source as isize;
        (delta != 0).then(|| format!("{delta:+}"))
    }
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

pub fn duplicate_keys(findings: &[Finding]) -> usize {
    let mut seen = BTreeSet::new();
    findings
        .iter()
        .filter(|value| !seen.insert(value.key.as_str()))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AnimationSnapshot, Snapshot};

    fn node(width: f64) -> NodeSnapshot {
        NodeSnapshot {
            id: "dialog".into(),
            path: "html>body>div".into(),
            parent: Some("body".into()),
            tag: "div".into(),
            text: "Sign in".into(),
            role: "dialog".into(),
            name: "Sign in".into(),
            attributes: BTreeMap::new(),
            rect: [0.0, 0.0, width, 200.0],
            style: BTreeMap::new(),
        }
    }

    fn snapshot(width: f64) -> Snapshot {
        Snapshot {
            url: "file:///fixture".into(),
            title: "Fixture".into(),
            document: [1440.0, 900.0],
            nodes: vec![node(width)],
            animations: Vec::<AnimationSnapshot>::new(),
            active: None,
            pixel_hash: String::new(),
            screenshot_png: String::new(),
            console_errors: Vec::new(),
            network_failures: Vec::new(),
            unexpected_requests: Vec::new(),
            pending_requests: 0,
        }
    }

    #[test]
    fn emits_one_geometry_root_difference() {
        let mut findings = BTreeMap::new();
        compare_snapshot(
            &Viewport {
                width: 1440,
                height: 900,
            },
            "base",
            "click:sign-in",
            &snapshot(480.0),
            &snapshot(456.0),
            &mut findings,
        );
        let values = findings.into_values().collect::<Vec<_>>();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].property, "width");
        assert_eq!(values[0].delta.as_deref(), Some("-24px"));
    }
}
