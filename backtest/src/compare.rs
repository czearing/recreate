use crate::model::{
    Artifact, Finding, MotionEvidence, NodeEvidence, RasterTileEvidence, Report, SCHEMA_VERSION,
    State, Status,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn artifacts(source: &Artifact, candidate: &Artifact, elapsed_ms: u128) -> Report {
    if let Err(error) = source.verify().and_then(|_| candidate.verify()) {
        return preparation_required(source, candidate, elapsed_ms, error.to_string());
    }
    let candidate_states = candidate
        .states
        .iter()
        .map(|state| ((state.viewport.width, state.scenario.as_str()), state))
        .collect::<BTreeMap<_, _>>();
    let mut findings = Vec::new();
    let mut seen_targets = BTreeSet::new();
    let mut suppressed = 0;
    for expected in &source.states {
        let Some(actual) =
            candidate_states.get(&(expected.viewport.width, expected.scenario.as_str()))
        else {
            findings.push(finding(
                expected, "state", "scenario", "present", "missing", None,
            ));
            continue;
        };
        if !expected.capture_complete || !actual.capture_complete {
            return inconclusive(
                source.digest.clone(),
                elapsed_ms,
                "incomplete state evidence".into(),
            );
        }
        if expected.scenario == "load" {
            continue;
        }
        for finding in compare_state(expected, actual) {
            let key = finding.key.clone();
            if seen_targets.insert(key) {
                findings.push(finding);
            } else {
                suppressed += 1;
            }
        }
    }
    for finding in compare_runtime(source, &candidate_states) {
        let key = finding.key.clone();
        if seen_targets.insert(key) {
            findings.push(finding);
        } else {
            suppressed += 1;
        }
    }
    if findings.iter().any(|finding| finding.property != "pixels") {
        let before = findings.len();
        findings.retain(|finding| finding.property != "pixels");
        suppressed += before - findings.len();
    }
    findings.sort_by(|left, right| {
        left.viewport
            .cmp(&right.viewport)
            .then_with(|| scenario_rank(&left.scenario).cmp(&scenario_rank(&right.scenario)))
            .then_with(|| left.target.cmp(&right.target))
    });
    Report {
        schema_version: SCHEMA_VERSION,
        status: if findings.is_empty() {
            Status::Pass
        } else {
            Status::Fail
        },
        findings,
        suppressed_duplicates: suppressed,
        elapsed_ms,
        source_digest: source.digest.clone(),
        candidate_digest: candidate.digest.clone(),
        diagnostic: None,
        scope: None,
        allowed: Vec::new(),
        unused_allowances: Vec::new(),
    }
}

pub fn artifacts_focused(
    source: &Artifact,
    candidate: &Artifact,
    elapsed_ms: u128,
    focus: &str,
) -> Report {
    let focus = focus.trim();
    if focus.is_empty() {
        return artifacts(source, candidate, elapsed_ms);
    }
    if let Err(error) = source.verify().and_then(|_| candidate.verify()) {
        return preparation_required(source, candidate, elapsed_ms, error.to_string());
    }
    let source_matches = focus_match_count(source, focus);
    let candidate_matches = focus_match_count(candidate, focus);
    if source_matches == 0 || candidate_matches == 0 {
        let mut report = preparation_required(
            source,
            candidate,
            elapsed_ms,
            format!(
                "focus {focus:?} matched {source_matches} source and {candidate_matches} candidate elements"
            ),
        );
        report.scope = Some(focus.into());
        return report;
    }
    let mut report = artifacts(source, candidate, elapsed_ms);
    let needle = focus.to_ascii_lowercase();
    report.findings.retain(|finding| {
        finding.target.to_ascii_lowercase().contains(&needle)
            || finding.line.to_ascii_lowercase().contains(&needle)
    });
    report.status = if report.findings.is_empty() {
        Status::Pass
    } else {
        Status::Fail
    };
    report.scope = Some(focus.into());
    report
}

fn focus_match_count(artifact: &Artifact, focus: &str) -> usize {
    let needle = focus.to_ascii_lowercase();
    artifact
        .states
        .iter()
        .filter(|state| state.scenario != "load")
        .flat_map(|state| {
            state.nodes.iter().filter(|(_, node)| node.visible).filter({
                let needle = needle.clone();
                move |(key, node)| {
                    key.to_ascii_lowercase().contains(&needle)
                        || semantic_target(state, key, node)
                            .to_ascii_lowercase()
                            .contains(&needle)
                        || semantic_text(node).to_ascii_lowercase().contains(&needle)
                }
            })
        })
        .count()
}

pub fn inconclusive(source_digest: String, elapsed_ms: u128, diagnostic: String) -> Report {
    Report {
        schema_version: SCHEMA_VERSION,
        status: Status::Inconclusive,
        findings: Vec::new(),
        suppressed_duplicates: 0,
        elapsed_ms,
        source_digest,
        candidate_digest: String::new(),
        diagnostic: Some(diagnostic),
        scope: None,
        allowed: Vec::new(),
        unused_allowances: Vec::new(),
    }
}

pub fn duplicate_keys(findings: &[Finding]) -> usize {
    let mut keys = BTreeSet::new();
    findings
        .iter()
        .filter(|finding| !keys.insert(finding.key.clone()))
        .count()
}

fn preparation_required(
    source: &Artifact,
    candidate: &Artifact,
    elapsed_ms: u128,
    diagnostic: String,
) -> Report {
    Report {
        schema_version: SCHEMA_VERSION,
        status: Status::PreparationRequired,
        findings: Vec::new(),
        suppressed_duplicates: 0,
        elapsed_ms,
        source_digest: source.digest.clone(),
        candidate_digest: candidate.digest.clone(),
        diagnostic: Some(diagnostic),
        scope: None,
        allowed: Vec::new(),
        unused_allowances: Vec::new(),
    }
}

pub fn preparation_required_session(
    source_digest: String,
    elapsed_ms: u128,
    diagnostic: String,
) -> Report {
    Report {
        schema_version: SCHEMA_VERSION,
        status: Status::PreparationRequired,
        findings: Vec::new(),
        suppressed_duplicates: 0,
        elapsed_ms,
        source_digest,
        candidate_digest: String::new(),
        diagnostic: Some(diagnostic),
        scope: None,
        allowed: Vec::new(),
        unused_allowances: Vec::new(),
    }
}

fn compare_state(source: &State, candidate: &State) -> Vec<Finding> {
    if has_authored_targets(source) || has_authored_targets(candidate) {
        compare_authored_state(source, candidate)
    } else {
        compare_semantic_state(source, candidate)
    }
}

fn has_authored_targets(state: &State) -> bool {
    state
        .nodes
        .keys()
        .any(|target| target != "html" && !target.contains('>'))
}

fn compare_authored_state(source: &State, candidate: &State) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut structural_roots = Vec::<String>::new();
    let targets = source
        .nodes
        .keys()
        .chain(candidate.nodes.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for target in targets {
        if structural_roots
            .iter()
            .any(|root| target.starts_with(&format!("{root}>")))
        {
            continue;
        }
        match (source.nodes.get(&target), candidate.nodes.get(&target)) {
            (Some(_), None) => {
                findings.push(finding(source, &target, "node", "present", "missing", None));
                structural_roots.push(target);
            }
            (None, Some(_)) => {
                findings.push(finding(source, &target, "node", "missing", "present", None));
                structural_roots.push(target);
            }
            (Some(expected), Some(actual)) => {
                if let Some(value) = compare_node(source, &target, expected, actual) {
                    if value.property == "parent" {
                        structural_roots.push(target.clone());
                    }
                    findings.push(value);
                }
            }
            (None, None) => {}
        }
    }
    if source.scenario.starts_with("click:") && source.active_element != candidate.active_element {
        findings.push(finding(
            source,
            "active-element",
            "focus",
            &source.active_element,
            &candidate.active_element,
            None,
        ));
    }
    compare_pending_work(source, candidate, &mut findings);
    if findings.iter().any(|finding| {
        !matches!(
            finding.property.as_str(),
            "x" | "y" | "height" | "transform" | "pixels"
        )
    }) {
        findings.retain(|finding| {
            !matches!(
                finding.property.as_str(),
                "x" | "y" | "height" | "transform" | "pixels"
            )
        });
    }
    if findings.is_empty()
        && !source.screenshot_sha256.is_empty()
        && source.screenshot_sha256 != candidate.screenshot_sha256
    {
        findings.extend(compare_raster_tiles(source, candidate, true));
    }
    findings
}

fn compare_semantic_state(source: &State, candidate: &State) -> Vec<Finding> {
    let source_nodes = semantic_nodes(source);
    let candidate_nodes = semantic_nodes(candidate);
    let mut matched = vec![false; candidate_nodes.len()];
    let mut layout_pairs = Vec::new();
    let mut findings = Vec::new();

    for (source_key, expected) in source_nodes {
        let exact = semantic_signature(expected);
        let candidate_index = candidate_nodes
            .iter()
            .enumerate()
            .filter(|(index, _)| !matched[*index])
            .filter(|(_, (_, actual))| semantic_signature(actual) == exact)
            .min_by(|(_, (_, left)), (_, (_, right))| {
                spatial_score(expected, left).total_cmp(&spatial_score(expected, right))
            })
            .map(|(index, _)| index)
            .or_else(|| {
                let source_text = semantic_text(expected);
                (!source_text.is_empty())
                    .then(|| {
                        candidate_nodes
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| !matched[*index])
                            .filter(|(_, (_, actual))| {
                                semantic_text(actual).eq_ignore_ascii_case(source_text)
                            })
                            .min_by(|(_, (_, left)), (_, (_, right))| {
                                spatial_score(expected, left)
                                    .total_cmp(&spatial_score(expected, right))
                            })
                            .map(|(index, _)| index)
                    })
                    .flatten()
            })
            .or_else(|| {
                candidate_nodes
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| !matched[*index])
                    .filter(|(_, (_, actual))| semantic_kind(expected) == semantic_kind(actual))
                    .map(|(index, (_, actual))| (index, spatial_score(expected, actual)))
                    .filter(|(index, score)| {
                        let actual = candidate_nodes[*index].1;
                        *score <= 64.0
                            && text_similarity(semantic_text(expected), semantic_text(actual))
                                >= 0.35
                    })
                    .min_by(|(_, left), (_, right)| left.total_cmp(right))
                    .map(|(index, _)| index)
            });
        let target = semantic_target(source, source_key, expected);
        if let Some(index) = candidate_index {
            matched[index] = true;
            let (_, actual) = candidate_nodes[index];
            layout_pairs.push(LayoutPair {
                source: expected,
                candidate: actual,
                region: semantic_region(source, expected),
                kind: semantic_kind(expected),
            });
            findings.extend(compare_semantic_node(source, &target, expected, actual));
        } else {
            findings.push(finding(
                source, &target, "content", "present", "missing", None,
            ));
        }
    }

    for (index, (candidate_key, actual)) in candidate_nodes.iter().enumerate() {
        if !matched[index] {
            findings.push(finding(
                source,
                &semantic_target(candidate, candidate_key, actual),
                "content",
                "missing",
                "present",
                None,
            ));
        }
    }
    compare_pending_work(source, candidate, &mut findings);
    let layout_findings = compare_layout_topology(source, &layout_pairs);
    for layout in &layout_findings {
        let Some((region, kind)) = layout
            .target
            .strip_suffix(" layout")
            .and_then(|target| target.rsplit_once(' '))
        else {
            continue;
        };
        findings.retain(|finding| {
            !matches!(finding.property.as_str(), "x" | "y" | "width" | "height")
                || (finding.target != region
                    && !finding.target.starts_with(&format!("{region} / {kind} ")))
        });
    }
    findings.extend(layout_findings);
    let mut findings = compact_semantic_findings(source, findings);
    if findings.is_empty()
        && !source.screenshot_sha256.is_empty()
        && source.screenshot_sha256 != candidate.screenshot_sha256
    {
        findings.extend(compare_raster_tiles(source, candidate, false));
    }
    findings
}

fn compare_raster_tiles(
    source: &State,
    candidate: &State,
    fallback_to_viewport: bool,
) -> Vec<Finding> {
    if source.raster_tiles.is_empty() || candidate.raster_tiles.is_empty() {
        return fallback_to_viewport
            .then(|| viewport_pixel_finding(source, candidate))
            .into_iter()
            .collect();
    }
    let candidate_tiles = candidate
        .raster_tiles
        .iter()
        .map(|tile| ((tile.x, tile.y, tile.width, tile.height), tile))
        .collect::<BTreeMap<_, _>>();
    let mut owners = BTreeMap::<String, String>::new();
    let mut has_unowned_change = false;
    for source_tile in &source.raster_tiles {
        let Some(candidate_tile) = candidate_tiles.get(&(
            source_tile.x,
            source_tile.y,
            source_tile.width,
            source_tile.height,
        )) else {
            has_unowned_change = true;
            continue;
        };
        if source_tile.sha256 == candidate_tile.sha256 {
            continue;
        }
        if let Some((target, property)) =
            raster_owner(source, source_tile).or_else(|| raster_owner(candidate, candidate_tile))
        {
            owners.entry(target).or_insert(property);
        } else {
            has_unowned_change = true;
        }
    }
    let mut findings = owners
        .into_iter()
        .map(|(target, property)| {
            finding(source, &target, &property, "expected", "different", None)
        })
        .collect::<Vec<_>>();
    if findings.is_empty() && has_unowned_change && fallback_to_viewport {
        findings.push(viewport_pixel_finding(source, candidate));
    }
    findings
}

fn viewport_pixel_finding(source: &State, candidate: &State) -> Finding {
    finding(
        source,
        "viewport",
        "pixels",
        &source.screenshot_sha256[..12],
        &candidate.screenshot_sha256[..12],
        None,
    )
}

fn raster_owner(state: &State, tile: &RasterTileEvidence) -> Option<(String, String)> {
    let left = f64::from(tile.x);
    let top = f64::from(tile.y);
    let right = left + f64::from(tile.width);
    let bottom = top + f64::from(tile.height);
    state
        .nodes
        .iter()
        .filter(|(_, node)| {
            node.visible
                && !node.raster_kind.is_empty()
                && node.width > 0.0
                && node.height > 0.0
                && node.x < right
                && node.y < bottom
                && node.x + node.width > left
                && node.y + node.height > top
        })
        .min_by(|(_, left), (_, right)| {
            (left.width * left.height).total_cmp(&(right.width * right.height))
        })
        .map(|(key, node)| (semantic_target(state, key, node), node.raster_kind.clone()))
}

struct LayoutPair<'a> {
    source: &'a NodeEvidence,
    candidate: &'a NodeEvidence,
    region: String,
    kind: &'a str,
}

#[derive(Default)]
struct LayoutMetrics {
    rows: usize,
    columns: usize,
    horizontal_gap: Option<f64>,
    vertical_gap: Option<f64>,
    overlaps: usize,
    order: Vec<usize>,
    density: f64,
}

fn compare_layout_topology(state: &State, pairs: &[LayoutPair<'_>]) -> Vec<Finding> {
    let mut cohorts = BTreeMap::<(String, String, i64, i64), Vec<usize>>::new();
    for (index, pair) in pairs.iter().enumerate() {
        if !matches!(
            pair.kind,
            "button" | "link" | "tab" | "option" | "listitem" | "article" | "image"
        ) || pair.source.width < 8.0
            || pair.source.height < 8.0
        {
            continue;
        }
        cohorts
            .entry((
                pair.region.clone(),
                pair.kind.into(),
                (pair.source.width / 32.0).round() as i64,
                (pair.source.height / 24.0).round() as i64,
            ))
            .or_default()
            .push(index);
    }

    let mut dominant = BTreeMap::<(String, String), Vec<usize>>::new();
    for ((region, kind, _, _), indices) in cohorts {
        if indices.len() < 3 {
            continue;
        }
        let key = (region, kind);
        if dominant
            .get(&key)
            .is_none_or(|current| indices.len() > current.len())
        {
            dominant.insert(key, indices);
        }
    }

    let mut findings = Vec::new();
    for ((region, kind), indices) in dominant {
        let source_metrics = layout_metrics(
            indices
                .iter()
                .map(|index| (*index, pairs[*index].source))
                .collect(),
        );
        let candidate_metrics = layout_metrics(
            indices
                .iter()
                .map(|index| (*index, pairs[*index].candidate))
                .collect(),
        );
        if source_metrics.density < 0.25 {
            continue;
        }
        let target = format!("{region} {kind} layout");
        if source_metrics.rows != candidate_metrics.rows
            || source_metrics.columns != candidate_metrics.columns
        {
            findings.push(finding(
                state,
                &target,
                "flow",
                &format!(
                    "{} columns / {} rows",
                    source_metrics.columns, source_metrics.rows
                ),
                &format!(
                    "{} columns / {} rows",
                    candidate_metrics.columns, candidate_metrics.rows
                ),
                None,
            ));
        } else if source_metrics.order != candidate_metrics.order {
            findings.push(finding(
                state,
                &target,
                "visual-order",
                "source order",
                "changed order",
                None,
            ));
        } else if source_metrics.overlaps != candidate_metrics.overlaps {
            findings.push(finding(
                state,
                &target,
                "overlaps",
                &source_metrics.overlaps.to_string(),
                &candidate_metrics.overlaps.to_string(),
                Some(signed_delta(
                    candidate_metrics.overlaps as i64 - source_metrics.overlaps as i64,
                )),
            ));
        } else if let Some((source_gap, candidate_gap)) = changed_gap(
            source_metrics.horizontal_gap,
            candidate_metrics.horizontal_gap,
        ) {
            findings.push(finding(
                state,
                &target,
                "column-gap",
                &format!("{}px", compact_number(source_gap)),
                &format!("{}px", compact_number(candidate_gap)),
                Some(format!("{}px", signed_number(candidate_gap - source_gap))),
            ));
        } else if let Some((source_gap, candidate_gap)) =
            changed_gap(source_metrics.vertical_gap, candidate_metrics.vertical_gap)
        {
            findings.push(finding(
                state,
                &target,
                "row-gap",
                &format!("{}px", compact_number(source_gap)),
                &format!("{}px", compact_number(candidate_gap)),
                Some(format!("{}px", signed_number(candidate_gap - source_gap))),
            ));
        }
    }
    findings
}

fn changed_gap(source: Option<f64>, candidate: Option<f64>) -> Option<(f64, f64)> {
    let (source, candidate) = (source?, candidate?);
    ((source - candidate).abs() >= 2.0).then_some((source, candidate))
}

fn layout_metrics(nodes: Vec<(usize, &NodeEvidence)>) -> LayoutMetrics {
    if nodes.is_empty() {
        return LayoutMetrics::default();
    }
    let mut rows = Vec::<Vec<(usize, &NodeEvidence)>>::new();
    let mut ordered = nodes.clone();
    ordered.sort_by(|(_, left), (_, right)| {
        left.y
            .total_cmp(&right.y)
            .then_with(|| left.x.total_cmp(&right.x))
    });
    for entry in ordered {
        let tolerance = (entry.1.height * 0.35).clamp(6.0, 20.0);
        if let Some(row) = rows.iter_mut().find(|row| {
            row.first()
                .is_some_and(|(_, first)| (first.y - entry.1.y).abs() <= tolerance)
        }) {
            row.push(entry);
        } else {
            rows.push(vec![entry]);
        }
    }
    for row in &mut rows {
        row.sort_by(|(_, left), (_, right)| left.x.total_cmp(&right.x));
    }
    rows.sort_by(|left, right| left[0].1.y.total_cmp(&right[0].1.y));

    let mut horizontal_gaps = Vec::new();
    for row in &rows {
        for pair in row.windows(2) {
            horizontal_gaps.push(pair[1].1.x - (pair[0].1.x + pair[0].1.width));
        }
    }
    let mut vertical_gaps = Vec::new();
    for pair in rows.windows(2) {
        let previous_bottom = pair[0]
            .iter()
            .map(|(_, node)| node.y + node.height)
            .fold(f64::NEG_INFINITY, f64::max);
        let next_top = pair[1]
            .iter()
            .map(|(_, node)| node.y)
            .fold(f64::INFINITY, f64::min);
        vertical_gaps.push(next_top - previous_bottom);
    }
    let mut overlaps = 0;
    for left in 0..nodes.len() {
        for right in (left + 1)..nodes.len() {
            if overlap_area(nodes[left].1, nodes[right].1) >= 4.0 {
                overlaps += 1;
            }
        }
    }
    let left = nodes
        .iter()
        .map(|(_, node)| node.x)
        .fold(f64::INFINITY, f64::min);
    let top = nodes
        .iter()
        .map(|(_, node)| node.y)
        .fold(f64::INFINITY, f64::min);
    let right = nodes
        .iter()
        .map(|(_, node)| node.x + node.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = nodes
        .iter()
        .map(|(_, node)| node.y + node.height)
        .fold(f64::NEG_INFINITY, f64::max);
    let bounds_area = (right - left).max(0.0) * (bottom - top).max(0.0);
    let element_area = nodes
        .iter()
        .map(|(_, node)| node.width * node.height)
        .sum::<f64>();
    LayoutMetrics {
        rows: rows.len(),
        columns: rows.iter().map(Vec::len).max().unwrap_or_default(),
        horizontal_gap: median(horizontal_gaps),
        vertical_gap: median(vertical_gaps),
        overlaps,
        order: rows.into_iter().flatten().map(|(index, _)| index).collect(),
        density: if bounds_area > 0.0 {
            (element_area / bounds_area).min(1.0)
        } else {
            0.0
        },
    }
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    values.retain(|value| value.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(values[values.len() / 2])
}

fn overlap_area(left: &NodeEvidence, right: &NodeEvidence) -> f64 {
    let width = (left.x + left.width).min(right.x + right.width) - left.x.max(right.x);
    let height = (left.y + left.height).min(right.y + right.height) - left.y.max(right.y);
    width.max(0.0) * height.max(0.0)
}

fn semantic_nodes(state: &State) -> Vec<(&str, &NodeEvidence)> {
    state
        .nodes
        .iter()
        .filter(|(_, node)| {
            node.visible
                && node.width > 0.0
                && node.height > 0.0
                && (!semantic_text(node).is_empty() || is_semantic_region(node))
        })
        .filter(|(key, node)| !duplicates_semantic_ancestor(state, key, node))
        .map(|(key, node)| (key.as_str(), node))
        .collect()
}

fn duplicates_semantic_ancestor(state: &State, _key: &str, node: &NodeEvidence) -> bool {
    if !matches!(
        semantic_kind(node),
        "text" | "generic" | "LabelText" | "paragraph"
    ) {
        return false;
    }
    let text = semantic_text(node);
    if text.is_empty() {
        return false;
    }
    let mut parent = node.parent.as_str();
    for _ in 0..4 {
        let Some(ancestor) = state.nodes.get(parent) else {
            break;
        };
        if semantic_text(ancestor).eq_ignore_ascii_case(text)
            && !matches!(semantic_kind(ancestor), "generic" | "div" | "span")
        {
            return true;
        }
        parent = ancestor.parent.as_str();
    }
    false
}

fn is_semantic_region(node: &NodeEvidence) -> bool {
    matches!(
        semantic_kind(node),
        "toolbar" | "navigation" | "banner" | "main" | "complementary" | "dialog"
    ) || matches!(node.tag.as_str(), "header" | "nav" | "main" | "aside")
}

fn semantic_signature(node: &NodeEvidence) -> String {
    format!(
        "{}:{}",
        semantic_kind(node),
        semantic_text(node).to_ascii_lowercase()
    )
}

fn semantic_kind(node: &NodeEvidence) -> &str {
    match node.role.as_str() {
        "" | "generic" | "none" => match node.tag.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
            "p" | "span" | "label" | "strong" | "em" | "output" => "text",
            value => value,
        },
        value => value,
    }
}

fn semantic_text(node: &NodeEvidence) -> &str {
    if node.accessible_name.trim().is_empty() {
        node.text.trim()
    } else {
        node.accessible_name.trim()
    }
}

fn spatial_score(source: &NodeEvidence, candidate: &NodeEvidence) -> f64 {
    let source_center = (
        source.x + source.width / 2.0,
        source.y + source.height / 2.0,
    );
    let candidate_center = (
        candidate.x + candidate.width / 2.0,
        candidate.y + candidate.height / 2.0,
    );
    let dx = source_center.0 - candidate_center.0;
    let dy = source_center.1 - candidate_center.1;
    let size = (source.width - candidate.width).abs() + (source.height - candidate.height).abs();
    dx.hypot(dy) + size * 0.25
}

fn text_similarity(source: &str, candidate: &str) -> f64 {
    let source = normalized_words(source);
    let candidate = normalized_words(candidate);
    if source.is_empty() || candidate.is_empty() {
        return 0.0;
    }
    let intersection = source.intersection(&candidate).count() as f64;
    let union = source.union(&candidate).count() as f64;
    intersection / union
}

fn normalized_words(value: &str) -> BTreeSet<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn semantic_target(state: &State, key: &str, node: &NodeEvidence) -> String {
    let own = semantic_description(node);
    let region = semantic_region(state, node);
    if own == region {
        return own;
    }
    if own == "element" {
        format!("element at {},{}", node.x.round(), node.y.round())
    } else if semantic_text(node).is_empty() && key == "html" {
        "page".into()
    } else if !is_semantic_region(node) {
        format!("{region} / {own}")
    } else {
        own
    }
}

fn semantic_region(state: &State, node: &NodeEvidence) -> String {
    let mut parent = node.parent.as_str();
    while let Some(ancestor) = state.nodes.get(parent) {
        let kind = semantic_kind(ancestor);
        if matches!(
            kind,
            "toolbar" | "navigation" | "banner" | "main" | "complementary" | "dialog"
        ) || matches!(ancestor.tag.as_str(), "header" | "nav" | "main" | "aside")
        {
            return semantic_description(ancestor);
        }
        parent = ancestor.parent.as_str();
    }
    inferred_region(node).into()
}

fn inferred_region(node: &NodeEvidence) -> &'static str {
    if node.y < 64.0 {
        "application toolbar"
    } else if node.y < 225.0 {
        "page header"
    } else {
        "content"
    }
}

fn semantic_description(node: &NodeEvidence) -> String {
    let kind = semantic_kind(node);
    let text = truncate(semantic_text(node), 72);
    if text.is_empty() {
        match kind {
            "generic" | "text" | "div" | "span" => "element".into(),
            value => value.replace('-', " "),
        }
    } else if matches!(kind, "text" | "generic" | "div" | "span") {
        format!("text {}", quoted(&text))
    } else {
        format!("{} {}", kind.replace('-', " "), quoted(&text))
    }
}

fn truncate(value: &str, maximum: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= maximum {
        compact
    } else {
        format!(
            "{}...",
            compact.chars().take(maximum - 3).collect::<String>()
        )
    }
}

fn compare_semantic_node(
    state: &State,
    target: &str,
    source: &NodeEvidence,
    candidate: &NodeEvidence,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let source_text = semantic_text(source);
    let candidate_text = semantic_text(candidate);
    if source_text != candidate_text {
        findings.push(finding(
            state,
            target,
            "text",
            &quoted(&truncate(source_text, 72)),
            &quoted(&truncate(candidate_text, 72)),
            None,
        ));
    }
    if semantic_kind(source) != semantic_kind(candidate) {
        findings.push(finding(
            state,
            target,
            "role",
            semantic_kind(source),
            semantic_kind(candidate),
            None,
        ));
    }
    if let Some(property) = rendered_content_property(source, candidate)
        && source.rendered_content_sha256 != candidate.rendered_content_sha256
    {
        findings.push(finding(
            state,
            target,
            property,
            "expected",
            "different",
            None,
        ));
    }

    if source.font_size != candidate.font_size {
        findings.push(finding(
            state,
            target,
            "font-size",
            &source.font_size,
            &candidate.font_size,
            css_pixel_delta(&source.font_size, &candidate.font_size),
        ));
    } else if source.font_weight != candidate.font_weight {
        findings.push(finding(
            state,
            target,
            "font-weight",
            &source.font_weight,
            &candidate.font_weight,
            None,
        ));
    } else if normalized_font_family(&source.font_family)
        != normalized_font_family(&candidate.font_family)
    {
        findings.push(finding(
            state,
            target,
            "font-family",
            &normalized_font_family(&source.font_family),
            &normalized_font_family(&candidate.font_family),
            None,
        ));
    } else if source.line_height != candidate.line_height {
        findings.push(finding(
            state,
            target,
            "line-height",
            &source.line_height,
            &candidate.line_height,
            css_pixel_delta(&source.line_height, &candidate.line_height),
        ));
    }

    for (property, expected, actual) in [
        ("x", source.x, candidate.x),
        ("y", source.y, candidate.y),
        ("width", source.width, candidate.width),
        ("height", source.height, candidate.height),
    ] {
        if (expected - actual).abs() >= 2.0 {
            findings.push(finding(
                state,
                target,
                property,
                &compact_number(expected),
                &compact_number(actual),
                Some(format!("{}px", signed_number(actual - expected))),
            ));
            break;
        }
    }

    if source.color != candidate.color {
        findings.push(finding(
            state,
            target,
            "color",
            &source.color,
            &candidate.color,
            None,
        ));
    } else if source.background != candidate.background {
        findings.push(finding(
            state,
            target,
            "background",
            &source.background,
            &candidate.background,
            None,
        ));
    }
    if let Some(motion) = compare_motion(state, target, source, candidate) {
        findings.push(motion);
    }
    findings
}

fn compare_motion(
    state: &State,
    target: &str,
    source: &NodeEvidence,
    candidate: &NodeEvidence,
) -> Option<Finding> {
    let source_motions = source
        .motions
        .iter()
        .map(|motion| (motion_identity(motion), motion))
        .collect::<BTreeMap<_, _>>();
    let candidate_motions = candidate
        .motions
        .iter()
        .map(|motion| (motion_identity(motion), motion))
        .collect::<BTreeMap<_, _>>();
    let identities = source_motions
        .keys()
        .chain(candidate_motions.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for identity in identities {
        let source_motion = source_motions.get(&identity).copied();
        let candidate_motion = candidate_motions.get(&identity).copied();
        let motion = source_motion.or(candidate_motion)?;
        let motion_target = format!(
            "{target} / {} {}",
            motion.kind.replace('-', " "),
            quoted(if motion.name.is_empty() {
                "unnamed"
            } else {
                &motion.name
            })
        );
        let (Some(source_motion), Some(candidate_motion)) = (source_motion, candidate_motion)
        else {
            return Some(finding(
                state,
                &motion_target,
                "motion",
                if source_motion.is_some() {
                    "present"
                } else {
                    "missing"
                },
                if candidate_motion.is_some() {
                    "present"
                } else {
                    "missing"
                },
                None,
            ));
        };
        if source_motion.duration_ms != candidate_motion.duration_ms {
            return Some(finding(
                state,
                &motion_target,
                "duration",
                &format!("{}ms", source_motion.duration_ms),
                &format!("{}ms", candidate_motion.duration_ms),
                Some(format!(
                    "{}ms",
                    signed_delta(
                        candidate_motion.duration_ms as i64 - source_motion.duration_ms as i64
                    )
                )),
            ));
        }
        for (property, source_value, candidate_value) in [
            (
                "delay",
                format!("{}ms", source_motion.delay_ms),
                format!("{}ms", candidate_motion.delay_ms),
            ),
            (
                "end-delay",
                format!("{}ms", source_motion.end_delay_ms),
                format!("{}ms", candidate_motion.end_delay_ms),
            ),
            (
                "iterations",
                source_motion.iterations.clone(),
                candidate_motion.iterations.clone(),
            ),
            (
                "direction",
                source_motion.direction.clone(),
                candidate_motion.direction.clone(),
            ),
            (
                "fill",
                source_motion.fill.clone(),
                candidate_motion.fill.clone(),
            ),
            (
                "easing",
                source_motion.easing.clone(),
                candidate_motion.easing.clone(),
            ),
        ] {
            if source_value != candidate_value {
                return Some(finding(
                    state,
                    &motion_target,
                    property,
                    &source_value,
                    &candidate_value,
                    None,
                ));
            }
        }
        for (source_checkpoint, candidate_checkpoint) in source_motion
            .checkpoints
            .iter()
            .zip(&candidate_motion.checkpoints)
        {
            if source_checkpoint.progress != candidate_checkpoint.progress {
                return Some(finding(
                    state,
                    &motion_target,
                    "checkpoints",
                    &source_checkpoint.progress.to_string(),
                    &candidate_checkpoint.progress.to_string(),
                    None,
                ));
            }
            let properties = source_checkpoint
                .values
                .keys()
                .chain(candidate_checkpoint.values.keys())
                .cloned()
                .collect::<BTreeSet<_>>();
            for property in properties {
                let source_value = source_checkpoint
                    .values
                    .get(&property)
                    .map(String::as_str)
                    .unwrap_or("missing");
                let candidate_value = candidate_checkpoint
                    .values
                    .get(&property)
                    .map(String::as_str)
                    .unwrap_or("missing");
                if source_value != candidate_value {
                    return Some(finding(
                        state,
                        &motion_target,
                        &format!("motion-{}%-{property}", source_checkpoint.progress),
                        &truncate(source_value, 48),
                        &truncate(candidate_value, 48),
                        None,
                    ));
                }
            }
        }
        if source_motion.checkpoints.len() != candidate_motion.checkpoints.len() {
            return Some(finding(
                state,
                &motion_target,
                "checkpoints",
                &source_motion.checkpoints.len().to_string(),
                &candidate_motion.checkpoints.len().to_string(),
                Some(signed_delta(
                    candidate_motion.checkpoints.len() as i64
                        - source_motion.checkpoints.len() as i64,
                )),
            ));
        }
    }
    None
}

fn motion_identity(motion: &MotionEvidence) -> String {
    format!("{}:{}", motion.kind, motion.properties.join(","))
}

fn compact_semantic_findings(state: &State, findings: Vec<Finding>) -> Vec<Finding> {
    let mut groups = BTreeMap::<(String, String, String, String, String), Vec<Finding>>::new();
    let mut content_groups = BTreeMap::<(String, String, String), Vec<Finding>>::new();
    let mut layout_groups = BTreeMap::<(String, String, String), Vec<Finding>>::new();
    let mut output = Vec::new();
    for finding in findings {
        let region = finding
            .target
            .split(" / ")
            .next()
            .unwrap_or(&finding.target)
            .to_string();
        if finding.property == "content" {
            let kind = finding
                .target
                .strip_prefix(&format!("{region} / "))
                .unwrap_or(&finding.target)
                .split_whitespace()
                .next()
                .unwrap_or("element")
                .to_string();
            let direction = if finding.source == "present" {
                "missing"
            } else {
                "unexpected"
            };
            content_groups
                .entry((region, kind, direction.into()))
                .or_default()
                .push(finding);
        } else if matches!(finding.property.as_str(), "x" | "y" | "width" | "height") {
            let delta = finding
                .line
                .split_whitespace()
                .last()
                .unwrap_or_default()
                .to_string();
            layout_groups
                .entry((region, finding.property.clone(), delta))
                .or_default()
                .push(finding);
        } else if matches!(
            finding.property.as_str(),
            "font-size" | "font-family" | "font-weight" | "line-height" | "color" | "background"
        ) {
            let delta = finding
                .line
                .split_whitespace()
                .last()
                .filter(|value| value.starts_with('+') || value.starts_with('-'))
                .unwrap_or_default()
                .to_string();
            groups
                .entry((
                    region,
                    finding.property.clone(),
                    finding.source.clone(),
                    finding.candidate.clone(),
                    delta,
                ))
                .or_default()
                .push(finding);
        } else {
            output.push(finding);
        }
    }
    for ((region, kind, direction), mut values) in content_groups {
        if values.len() >= 2 {
            let labels = values
                .iter()
                .map(|value| {
                    value
                        .target
                        .strip_prefix(&format!("{region} / "))
                        .unwrap_or(&value.target)
                        .to_string()
                })
                .collect::<Vec<_>>();
            let line = format!(
                "V{} {} {} {} {} {}: {}",
                state.viewport.width,
                state.scenario,
                region,
                kind,
                direction,
                values.len(),
                labels.join(", ")
            );
            output.push(summary_finding(
                state,
                &format!("{region} {kind}"),
                "content",
                &line,
            ));
        } else {
            output.append(&mut values);
        }
    }
    for ((region, property, delta), mut values) in layout_groups {
        if values.len() >= 3 {
            let labels = grouped_labels(&region, &values);
            let line = format!(
                "V{} {} {} layout {} shifted {} ({} elements: {})",
                state.viewport.width,
                state.scenario,
                region,
                property,
                delta,
                values.len(),
                labels.join(", ")
            );
            output.push(summary_finding(
                state,
                &format!("{region} layout"),
                &property,
                &line,
            ));
        } else {
            output.append(&mut values);
        }
    }
    for ((region, property, source, candidate, delta), mut values) in groups {
        if values.len() >= 3 {
            let labels = grouped_labels(&region, &values);
            let suffix = if delta.is_empty() {
                String::new()
            } else {
                format!(" {delta}")
            };
            let line = format!(
                "V{} {} {} {} {}->{}{} ({} elements: {})",
                state.viewport.width,
                state.scenario,
                region,
                property,
                source,
                candidate,
                suffix,
                values.len(),
                labels.join(", ")
            );
            output.push(summary_finding(
                state,
                &format!("{region} {property}"),
                &property,
                &line,
            ));
        } else {
            output.append(&mut values);
        }
    }
    output
}

fn grouped_labels(region: &str, values: &[Finding]) -> Vec<String> {
    values
        .iter()
        .map(|value| {
            value
                .target
                .strip_prefix(&format!("{region} / "))
                .unwrap_or(&value.target)
                .to_string()
        })
        .collect()
}

fn summary_finding(state: &State, target: &str, property: &str, line: &str) -> Finding {
    Finding {
        key: format!("{}:{}:{}", state.viewport.width, target, property),
        line: line.into(),
        viewport: state.viewport.width,
        scenario: state.scenario.clone(),
        target: target.into(),
        property: property.into(),
        source: String::new(),
        candidate: String::new(),
        severity: "error".into(),
        confidence: "exact".into(),
    }
}

fn normalized_font_family(value: &str) -> String {
    value
        .split(',')
        .next()
        .unwrap_or(value)
        .trim()
        .trim_matches('"')
        .to_string()
}

fn css_pixel_delta(source: &str, candidate: &str) -> Option<String> {
    let source = source.strip_suffix("px")?.parse::<f64>().ok()?;
    let candidate = candidate.strip_suffix("px")?.parse::<f64>().ok()?;
    Some(format!("{}px", signed_number(candidate - source)))
}

fn compare_pending_work(source: &State, candidate: &State, findings: &mut Vec<Finding>) {
    if source.runtime.pending_timers != candidate.runtime.pending_timers {
        findings.push(finding(
            source,
            "runtime",
            "pending-timers",
            &source.runtime.pending_timers.to_string(),
            &candidate.runtime.pending_timers.to_string(),
            Some(signed_delta(
                candidate.runtime.pending_timers as i64 - source.runtime.pending_timers as i64,
            )),
        ));
    }
    if source.runtime.pending_frames != candidate.runtime.pending_frames {
        findings.push(finding(
            source,
            "runtime",
            "pending-frames",
            &source.runtime.pending_frames.to_string(),
            &candidate.runtime.pending_frames.to_string(),
            Some(signed_delta(
                candidate.runtime.pending_frames as i64 - source.runtime.pending_frames as i64,
            )),
        ));
    }
    let source_shift = layout_shift_total(source);
    let candidate_shift = layout_shift_total(candidate);
    if (source_shift - candidate_shift).abs() >= 0.001 {
        let label = source
            .runtime
            .layout_shifts
            .iter()
            .chain(&candidate.runtime.layout_shifts)
            .flat_map(|shift| &shift.sources)
            .next()
            .map(|source| format!("layout near {source}"))
            .unwrap_or_else(|| "layout".into());
        findings.push(finding(
            source,
            &label,
            "shift-score",
            &compact_number(source_shift),
            &compact_number(candidate_shift),
            Some(signed_number(candidate_shift - source_shift)),
        ));
    }
}

fn layout_shift_total(state: &State) -> f64 {
    state
        .runtime
        .layout_shifts
        .iter()
        .map(|shift| shift.value)
        .sum()
}

fn compare_runtime(
    source: &Artifact,
    candidate_states: &BTreeMap<(u32, &str), &State>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for expected in source
        .states
        .iter()
        .filter(|state| state.scenario == "load")
    {
        let Some(actual) =
            candidate_states.get(&(expected.viewport.width, expected.scenario.as_str()))
        else {
            continue;
        };
        compare_load_events(expected, actual, &mut findings);
    }
    for expected in source
        .states
        .iter()
        .filter(|state| state.scenario != "base" && state.scenario != "load")
    {
        let Some(actual) =
            candidate_states.get(&(expected.viewport.width, expected.scenario.as_str()))
        else {
            continue;
        };
        let source_load = source.states.iter().find(|state| {
            state.viewport.width == expected.viewport.width && state.scenario == "load"
        });
        let candidate_load = candidate_states
            .get(&(expected.viewport.width, "load"))
            .copied();
        compare_interaction_events(expected, actual, source_load, candidate_load, &mut findings);
    }
    findings
}

fn compare_load_events(source: &State, candidate: &State, findings: &mut Vec<Finding>) {
    for (target, property, expected, actual) in [
        (
            "window",
            "console-errors",
            source.runtime.console_errors.as_slice(),
            candidate.runtime.console_errors.as_slice(),
        ),
        (
            "network",
            "unexpected-requests",
            source.runtime.requests.as_slice(),
            candidate.runtime.requests.as_slice(),
        ),
    ] {
        if expected == actual {
            continue;
        }
        if expected.len() != actual.len() {
            findings.push(finding(
                source,
                target,
                property,
                &expected.len().to_string(),
                &actual.len().to_string(),
                Some(signed_delta(actual.len() as i64 - expected.len() as i64)),
            ));
        } else {
            push_event_values(source, target, property, expected, actual, findings);
        }
    }
}

fn compare_interaction_events(
    source: &State,
    candidate: &State,
    source_load: Option<&State>,
    candidate_load: Option<&State>,
    findings: &mut Vec<Finding>,
) {
    let source_errors = event_suffix(
        &source.runtime.console_errors,
        source_load
            .map(|state| state.runtime.console_errors.as_slice())
            .unwrap_or_default(),
    );
    let candidate_errors = event_suffix(
        &candidate.runtime.console_errors,
        candidate_load
            .map(|state| state.runtime.console_errors.as_slice())
            .unwrap_or_default(),
    );
    push_event_values(
        source,
        "window",
        "console-errors",
        source_errors,
        candidate_errors,
        findings,
    );
    let source_requests = event_suffix(
        &source.runtime.requests,
        source_load
            .map(|state| state.runtime.requests.as_slice())
            .unwrap_or_default(),
    );
    let candidate_requests = event_suffix(
        &candidate.runtime.requests,
        candidate_load
            .map(|state| state.runtime.requests.as_slice())
            .unwrap_or_default(),
    );
    push_event_values(
        source,
        "network",
        "unexpected-requests",
        source_requests,
        candidate_requests,
        findings,
    );
}

fn event_suffix<'a>(events: &'a [String], baseline: &[String]) -> &'a [String] {
    events.strip_prefix(baseline).unwrap_or(events)
}

fn push_event_values(
    state: &State,
    target: &str,
    property: &str,
    source: &[String],
    candidate: &[String],
    findings: &mut Vec<Finding>,
) {
    if source == candidate {
        return;
    }
    let expected = source.join("|");
    let actual = candidate.join("|");
    findings.push(finding(
        state,
        target,
        property,
        if expected.is_empty() { "0" } else { &expected },
        if actual.is_empty() { "0" } else { &actual },
        (source.len() != candidate.len())
            .then(|| signed_delta(candidate.len() as i64 - source.len() as i64)),
    ));
}

fn compare_node(
    state: &State,
    target: &str,
    source: &NodeEvidence,
    candidate: &NodeEvidence,
) -> Option<Finding> {
    if source.tag != candidate.tag {
        return Some(finding(
            state,
            target,
            "node",
            &source.tag,
            &candidate.tag,
            None,
        ));
    }
    if source.parent != candidate.parent {
        return Some(finding(
            state,
            target,
            "parent",
            &source.parent,
            &candidate.parent,
            None,
        ));
    }
    if source.order != candidate.order {
        return Some(finding(
            state,
            target,
            "order",
            &source.order.to_string(),
            &candidate.order.to_string(),
            None,
        ));
    }
    if source.text != candidate.text {
        return Some(finding(
            state,
            target,
            "text",
            &quoted(&source.text),
            &quoted(&candidate.text),
            None,
        ));
    }
    if source.visible != candidate.visible {
        return Some(finding(
            state,
            target,
            "visibility",
            visibility(source.visible),
            visibility(candidate.visible),
            None,
        ));
    }
    if source.role != candidate.role {
        return Some(finding(
            state,
            target,
            "role",
            &source.role,
            &candidate.role,
            None,
        ));
    }
    if source.accessible_name != candidate.accessible_name {
        return Some(finding(
            state,
            target,
            "name",
            &quoted(&source.accessible_name),
            &quoted(&candidate.accessible_name),
            None,
        ));
    }
    if let Some(property) = rendered_content_property(source, candidate)
        && source.rendered_content_sha256 != candidate.rendered_content_sha256
    {
        return Some(finding(
            state,
            target,
            property,
            "expected",
            "different",
            None,
        ));
    }
    if state.scenario.starts_with("animation:") {
        if source.animation_duration_ms != candidate.animation_duration_ms {
            return Some(finding(
                state,
                target,
                "duration",
                &format!("{}ms", source.animation_duration_ms.unwrap_or_default()),
                &format!("{}ms", candidate.animation_duration_ms.unwrap_or_default()),
                Some(format!(
                    "{}ms",
                    signed_delta(
                        candidate.animation_duration_ms.unwrap_or_default() as i64
                            - source.animation_duration_ms.unwrap_or_default() as i64
                    )
                )),
            ));
        }
        if source.animation_delay_ms != candidate.animation_delay_ms {
            return Some(finding(
                state,
                target,
                "delay",
                &format!("{}ms", source.animation_delay_ms.unwrap_or_default()),
                &format!("{}ms", candidate.animation_delay_ms.unwrap_or_default()),
                Some(format!(
                    "{}ms",
                    signed_delta(
                        candidate.animation_delay_ms.unwrap_or_default()
                            - source.animation_delay_ms.unwrap_or_default()
                    )
                )),
            ));
        }
        if source.animation_easing != candidate.animation_easing {
            return Some(finding(
                state,
                target,
                "easing",
                &source.animation_easing,
                &candidate.animation_easing,
                None,
            ));
        }
        if source.animation_direction != candidate.animation_direction {
            return Some(finding(
                state,
                target,
                "direction",
                &source.animation_direction,
                &candidate.animation_direction,
                None,
            ));
        }
    }
    if source.width > 0.0 && source.height > 0.0 && candidate.width > 0.0 && candidate.height > 0.0
    {
        let mut geometry = vec![
            ("width", source.width, candidate.width),
            ("height", source.height, candidate.height),
        ];
        if (!source.animated && !candidate.animated) || state.scenario.starts_with("animation:") {
            geometry.splice(
                0..0,
                [("x", source.x, candidate.x), ("y", source.y, candidate.y)],
            );
        }
        for (property, expected, actual) in geometry {
            if (expected - actual).abs() >= 0.5 {
                return Some(finding(
                    state,
                    target,
                    property,
                    &compact_number(expected),
                    &compact_number(actual),
                    Some(format!("{}px", signed_number(actual - expected))),
                ));
            }
        }
    }
    if source.background != candidate.background {
        return Some(finding(
            state,
            target,
            "background",
            &source.background,
            &candidate.background,
            None,
        ));
    }
    if source.font_weight != candidate.font_weight {
        return Some(finding(
            state,
            target,
            "font-weight",
            &source.font_weight,
            &candidate.font_weight,
            None,
        ));
    }
    for (property, expected, actual) in [
        ("color", &source.color, &candidate.color),
        (
            "border-color",
            &source.border_color,
            &candidate.border_color,
        ),
        (
            "border-radius",
            &source.border_radius,
            &candidate.border_radius,
        ),
        ("box-shadow", &source.box_shadow, &candidate.box_shadow),
        ("opacity", &source.opacity, &candidate.opacity),
        (
            "transform",
            if source.animated && !state.scenario.starts_with("animation:") {
                &candidate.transform
            } else {
                &source.transform
            },
            &candidate.transform,
        ),
    ] {
        if expected != actual {
            return Some(finding(state, target, property, expected, actual, None));
        }
    }
    if source.animation_duration_ms != candidate.animation_duration_ms {
        return Some(finding(
            state,
            target,
            "duration",
            &format!("{}ms", source.animation_duration_ms.unwrap_or_default()),
            &format!("{}ms", candidate.animation_duration_ms.unwrap_or_default()),
            Some(format!(
                "{}ms",
                signed_delta(
                    candidate.animation_duration_ms.unwrap_or_default() as i64
                        - source.animation_duration_ms.unwrap_or_default() as i64
                )
            )),
        ));
    }
    if source.animation_delay_ms != candidate.animation_delay_ms {
        return Some(finding(
            state,
            target,
            "delay",
            &format!("{}ms", source.animation_delay_ms.unwrap_or_default()),
            &format!("{}ms", candidate.animation_delay_ms.unwrap_or_default()),
            Some(format!(
                "{}ms",
                signed_delta(
                    candidate.animation_delay_ms.unwrap_or_default()
                        - source.animation_delay_ms.unwrap_or_default()
                )
            )),
        ));
    }
    if source.animation_easing != candidate.animation_easing {
        return Some(finding(
            state,
            target,
            "easing",
            &source.animation_easing,
            &candidate.animation_easing,
            None,
        ));
    }
    if source.animation_direction != candidate.animation_direction {
        return Some(finding(
            state,
            target,
            "direction",
            &source.animation_direction,
            &candidate.animation_direction,
            None,
        ));
    }
    compare_motion(state, target, source, candidate)
}

fn rendered_content_property(
    source: &NodeEvidence,
    candidate: &NodeEvidence,
) -> Option<&'static str> {
    if source.rendered_content_sha256.is_empty()
        || candidate.rendered_content_sha256.is_empty()
        || source.tag != candidate.tag
    {
        return None;
    }
    match source.tag.as_str() {
        "img" => Some("image-content"),
        "svg" => Some("svg-content"),
        _ => None,
    }
}

fn finding(
    state: &State,
    target: &str,
    property: &str,
    source: &str,
    candidate: &str,
    delta: Option<String>,
) -> Finding {
    let suffix = delta.map_or_else(String::new, |value| format!(" {value}"));
    let line = format!(
        "V{} {} {} {} {}->{}{}",
        state.viewport.width, state.scenario, target, property, source, candidate, suffix
    );
    Finding {
        key: format!("{}:{}:{}", state.viewport.width, target, property),
        line,
        viewport: state.viewport.width,
        scenario: state.scenario.clone(),
        target: target.into(),
        property: property.into(),
        source: source.into(),
        candidate: candidate.into(),
        severity: "error".into(),
        confidence: "exact".into(),
    }
}

fn visibility(value: bool) -> &'static str {
    if value { "visible" } else { "hidden" }
}

fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn compact_number(value: f64) -> String {
    if value.fract().abs() < 0.01 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.2}")
    }
}

fn signed_number(value: f64) -> String {
    let value = compact_number(value);
    if value.starts_with('-') {
        value
    } else {
        format!("+{value}")
    }
}

fn signed_delta(value: i64) -> String {
    if value >= 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

fn scenario_rank(value: &str) -> u8 {
    match value {
        "base" => 0,
        value if value.starts_with("click:") => 1,
        value if value.starts_with("hover:") => 2,
        value if value.starts_with("animation:") => 3,
        value if value.starts_with("timer:") => 4,
        "load" => 5,
        _ => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        LayoutShiftEvidence, MotionCheckpoint, MotionEvidence, RuntimeEvidence, SourceIdentity,
        Viewport,
    };
    use std::time::Instant;

    fn state(scenario: &str, node: NodeEvidence) -> State {
        State {
            viewport: Viewport {
                width: 1440,
                height: 900,
            },
            scenario: scenario.into(),
            nodes: BTreeMap::from([("target".into(), node)]),
            active_element: String::new(),
            runtime: RuntimeEvidence::default(),
            screenshot_sha256: String::new(),
            raster_tiles: Vec::new(),
            capture_complete: true,
        }
    }

    #[test]
    fn text_is_canonical_over_geometry() {
        let source = state(
            "base",
            NodeEvidence {
                text: "source".into(),
                width: 100.0,
                ..NodeEvidence::default()
            },
        );
        let candidate = state(
            "base",
            NodeEvidence {
                text: "candidate".into(),
                width: 120.0,
                ..NodeEvidence::default()
            },
        );
        let findings = compare_authored_state(&source, &candidate);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].property, "text");
    }

    #[test]
    fn authored_image_content_difference_is_actionable() {
        let source = state(
            "base",
            NodeEvidence {
                tag: "img".into(),
                rendered_content_sha256: "source".into(),
                ..NodeEvidence::default()
            },
        );
        let candidate = state(
            "base",
            NodeEvidence {
                tag: "img".into(),
                rendered_content_sha256: "candidate".into(),
                ..NodeEvidence::default()
            },
        );
        let findings = compare_authored_state(&source, &candidate);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].property, "image-content");
        assert_eq!(
            findings[0].line,
            "V1440 base target image-content expected->different"
        );
    }

    #[test]
    fn semantic_svg_content_difference_is_actionable() {
        let source = state(
            "base",
            NodeEvidence {
                tag: "svg".into(),
                role: "image".into(),
                accessible_name: "Brand mark".into(),
                visible: true,
                width: 48.0,
                height: 48.0,
                rendered_content_sha256: "source".into(),
                ..NodeEvidence::default()
            },
        );
        let candidate = state(
            "base",
            NodeEvidence {
                tag: "svg".into(),
                role: "image".into(),
                accessible_name: "Brand mark".into(),
                visible: true,
                width: 48.0,
                height: 48.0,
                rendered_content_sha256: "candidate".into(),
                ..NodeEvidence::default()
            },
        );
        let findings = compare_semantic_state(&source, &candidate);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].property, "svg-content");
        assert_eq!(
            findings[0].line,
            "V1440 base application toolbar / image \"Brand mark\" svg-content expected->different"
        );
    }

    #[test]
    fn identical_rendered_asset_content_is_ignored() {
        let source = NodeEvidence {
            tag: "img".into(),
            rendered_content_sha256: "same".into(),
            ..NodeEvidence::default()
        };
        assert!(compare_node(&state("base", source.clone()), "image", &source, &source).is_none());
    }

    #[test]
    fn interaction_runtime_values_are_compared() {
        let source = state("click:button", NodeEvidence::default());
        let mut candidate = state("click:button", NodeEvidence::default());
        candidate.runtime.console_errors = vec!["interaction failed".into()];
        let mut findings = Vec::new();
        compare_interaction_events(&source, &candidate, None, None, &mut findings);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].property, "console-errors");
        assert_eq!(findings[0].candidate, "interaction failed");
    }

    #[test]
    fn pending_work_is_compared_in_base_state() {
        let source = state("base", NodeEvidence::default());
        let mut candidate = state("base", NodeEvidence::default());
        candidate.runtime.pending_timers = 1;
        let findings = compare_state(&source, &candidate);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].property, "pending-timers");
    }

    #[test]
    fn unexpected_layout_shift_is_compared() {
        let mut source = state("timer:100", NodeEvidence::default());
        source.runtime.layout_shifts = vec![LayoutShiftEvidence {
            value: 0.12,
            sources: vec!["main \"Notebook grid\"".into()],
        }];
        let candidate = state("timer:100", NodeEvidence::default());
        let findings = compare_state(&source, &candidate);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].line,
            "V1440 timer:100 layout near main \"Notebook grid\" shift-score 0.12->0 -0.12"
        );
    }

    #[test]
    fn equal_count_load_event_values_are_compared() {
        let mut source = state("load", NodeEvidence::default());
        source.runtime.console_errors = vec!["source error".into()];
        let mut candidate = state("load", NodeEvidence::default());
        candidate.runtime.console_errors = vec!["candidate error".into()];
        let mut findings = Vec::new();
        compare_load_events(&source, &candidate, &mut findings);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].source, "source error");
        assert_eq!(findings[0].candidate, "candidate error");
    }

    #[test]
    fn interaction_pixel_only_difference_is_compared() {
        let mut source = state("hover:target", NodeEvidence::default());
        source.screenshot_sha256 = "source-pixels".into();
        let mut candidate = state("hover:target", NodeEvidence::default());
        candidate.screenshot_sha256 = "candidate-pixels".into();
        let findings = compare_state(&source, &candidate);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].property, "pixels");
    }

    #[test]
    fn grouped_content_findings_name_every_item() {
        let state = state("base", NodeEvidence::default());
        let findings = (1..=6)
            .map(|index| {
                finding(
                    &state,
                    &format!("main / button \"Item {index}\""),
                    "content",
                    "present",
                    "missing",
                    None,
                )
            })
            .collect();
        let compacted = compact_semantic_findings(&state, findings);
        assert_eq!(compacted.len(), 1);
        for index in 1..=6 {
            assert!(compacted[0].line.contains(&format!("Item {index}")));
        }
        assert!(!compacted[0].line.contains("more"));
    }

    #[test]
    fn grouped_layout_and_style_findings_name_every_item() {
        let state = state("base", NodeEvidence::default());
        for property in ["x", "background"] {
            let findings = (1..=6)
                .map(|index| {
                    finding(
                        &state,
                        &format!("main / button \"Item {index}\""),
                        property,
                        if property == "x" { "10" } else { "#ffffff" },
                        if property == "x" { "20" } else { "#000000" },
                        (property == "x").then(|| "+10px".into()),
                    )
                })
                .collect();
            let compacted = compact_semantic_findings(&state, findings);
            assert_eq!(compacted.len(), 1);
            for index in 1..=6 {
                assert!(compacted[0].line.contains(&format!("Item {index}")));
            }
        }
    }

    #[test]
    fn raster_attribution_reports_every_changed_owner() {
        let mut source = state("base", NodeEvidence::default());
        source.nodes.clear();
        source.screenshot_sha256 = "source-pixels".into();
        source.raster_tiles.clear();
        let mut candidate = source.clone();
        candidate.screenshot_sha256 = "candidate-pixels".into();
        for index in 0..12 {
            let x = index * 32;
            let target = format!("surface-{index}");
            source.nodes.insert(
                target.clone(),
                NodeEvidence {
                    tag: "section".into(),
                    visible: true,
                    x: f64::from(x),
                    width: 32.0,
                    height: 32.0,
                    accessible_name: format!("Surface {index}"),
                    raster_kind: "background-content".into(),
                    ..NodeEvidence::default()
                },
            );
            candidate.nodes.insert(
                target,
                NodeEvidence {
                    tag: "section".into(),
                    visible: true,
                    x: f64::from(x),
                    width: 32.0,
                    height: 32.0,
                    accessible_name: format!("Surface {index}"),
                    raster_kind: "background-content".into(),
                    ..NodeEvidence::default()
                },
            );
            source.raster_tiles.push(RasterTileEvidence {
                x,
                y: 0,
                width: 32,
                height: 32,
                sha256: format!("source-{index}"),
            });
            candidate.raster_tiles.push(RasterTileEvidence {
                x,
                y: 0,
                width: 32,
                height: 32,
                sha256: format!("candidate-{index}"),
            });
        }
        let findings = compare_raster_tiles(&source, &candidate, true);
        assert_eq!(findings.len(), 12);
    }

    #[test]
    fn structural_root_suppresses_descendant_noise() {
        let mut source = state("base", NodeEvidence::default());
        source.nodes = BTreeMap::from([
            ("html>body".into(), NodeEvidence::default()),
            ("html>body>main".into(), NodeEvidence::default()),
        ]);
        let mut candidate = state("base", NodeEvidence::default());
        candidate.nodes.clear();
        let findings = compare_authored_state(&source, &candidate);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].target, "html>body");
    }

    fn semantic_layout_state(columns: usize) -> State {
        let mut nodes = BTreeMap::from([(
            "html>body>main".into(),
            NodeEvidence {
                tag: "main".into(),
                role: "main".into(),
                visible: true,
                width: 800.0,
                height: 600.0,
                ..NodeEvidence::default()
            },
        )]);
        for index in 0..4 {
            let column = index % columns;
            let row = index / columns;
            nodes.insert(
                format!("html>body>main>button:nth-of-type({})", index + 1),
                NodeEvidence {
                    tag: "button".into(),
                    parent: "html>body>main".into(),
                    role: "button".into(),
                    accessible_name: format!("Card {}", index + 1),
                    visible: true,
                    x: 100.0 + column as f64 * 220.0,
                    y: 260.0 + row as f64 * 120.0,
                    width: 200.0,
                    height: 100.0,
                    ..NodeEvidence::default()
                },
            );
        }
        State {
            viewport: Viewport {
                width: 1440,
                height: 900,
            },
            scenario: "base".into(),
            nodes,
            active_element: String::new(),
            runtime: RuntimeEvidence::default(),
            screenshot_sha256: String::new(),
            raster_tiles: Vec::new(),
            capture_complete: true,
        }
    }

    #[test]
    fn semantic_layout_reports_flow_instead_of_coordinate_noise() {
        let source = semantic_layout_state(2);
        let candidate = semantic_layout_state(1);
        let findings = compare_semantic_state(&source, &candidate);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].line,
            "V1440 base main button layout flow 2 columns / 2 rows->1 columns / 4 rows"
        );
    }

    #[test]
    fn motion_duration_is_reported_without_waiting() {
        let motion = |duration_ms| MotionEvidence {
            kind: "web-animation".into(),
            name: "panel-open".into(),
            duration_ms,
            properties: vec!["opacity".into()],
            checkpoints: vec![MotionCheckpoint {
                progress: 50,
                values: BTreeMap::from([("opacity".into(), "0.5".into())]),
            }],
            ..MotionEvidence::default()
        };
        let source = state(
            "base",
            NodeEvidence {
                motions: vec![motion(200)],
                ..NodeEvidence::default()
            },
        );
        let candidate = state(
            "base",
            NodeEvidence {
                motions: vec![motion(350)],
                ..NodeEvidence::default()
            },
        );
        let finding = compare_motion(
            &source,
            "panel",
            &source.nodes["target"],
            &candidate.nodes["target"],
        )
        .unwrap();
        assert_eq!(
            finding.line,
            "V1440 base panel / web animation \"panel-open\" duration 200ms->350ms +150ms"
        );
    }

    fn artifact(states: Vec<State>) -> Artifact {
        let mut artifact = Artifact {
            schema_version: SCHEMA_VERSION,
            source: SourceIdentity {
                requested_url: "http://fixture.test".into(),
                rendered_url: "http://fixture.test".into(),
                browser: "browser".into(),
                fingerprint: "fingerprint".into(),
            },
            actions: Vec::new(),
            states,
            digest: String::new(),
        };
        artifact.seal().unwrap();
        artifact
    }

    #[test]
    fn suppresses_repeated_root_difference() {
        let source_node = NodeEvidence {
            width: 100.0,
            height: 10.0,
            ..NodeEvidence::default()
        };
        let candidate_node = NodeEvidence {
            width: 80.0,
            height: 10.0,
            ..NodeEvidence::default()
        };
        let source = artifact(vec![
            state("base", source_node.clone()),
            state("click:button", source_node),
        ]);
        let candidate = artifact(vec![
            state("base", candidate_node.clone()),
            state("click:button", candidate_node),
        ]);
        let report = artifacts(&source, &candidate, 1);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.suppressed_duplicates, 1);
        assert_eq!(duplicate_keys(&report.findings), 0);
    }

    #[test]
    fn corrupted_artifact_fails_closed() {
        let mut source = artifact(vec![state("base", NodeEvidence::default())]);
        let candidate = source.clone();
        source.states[0].scenario = "corrupt".into();
        let report = artifacts(&source, &candidate, 1);
        assert_eq!(report.status, Status::PreparationRequired);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn focused_comparison_filters_to_semantic_region() {
        let mut source_state = state(
            "base",
            NodeEvidence {
                tag: "button".into(),
                role: "button".into(),
                accessible_name: "App launcher".into(),
                visible: true,
                width: 40.0,
                height: 40.0,
                color: "#000000".into(),
                ..NodeEvidence::default()
            },
        );
        source_state.nodes = BTreeMap::from([(
            "html>body>button:nth-of-type(1)".into(),
            source_state.nodes.remove("target").unwrap(),
        )]);
        let source = artifact(vec![source_state]);
        let mut candidate_state = state(
            "base",
            NodeEvidence {
                tag: "button".into(),
                role: "button".into(),
                accessible_name: "App launcher".into(),
                visible: true,
                width: 40.0,
                height: 40.0,
                color: "#ffffff".into(),
                ..NodeEvidence::default()
            },
        );
        candidate_state.nodes = BTreeMap::from([(
            "html>body>button:nth-of-type(1)".into(),
            candidate_state.nodes.remove("target").unwrap(),
        )]);
        let candidate = artifact(vec![candidate_state]);
        let report = artifacts_focused(&source, &candidate, 1, "toolbar");
        assert_eq!(report.status, Status::Fail);
        assert_eq!(report.scope.as_deref(), Some("toolbar"));
        assert_eq!(report.findings.len(), 1);
        assert!(report.findings[0].target.contains("App launcher"));
    }

    #[test]
    fn focused_comparison_fails_closed_when_candidate_has_no_match() {
        let source = artifact(vec![state(
            "base",
            NodeEvidence {
                tag: "button".into(),
                role: "button".into(),
                accessible_name: "App launcher".into(),
                visible: true,
                width: 40.0,
                height: 40.0,
                ..NodeEvidence::default()
            },
        )]);
        let candidate = artifact(vec![state("base", NodeEvidence::default())]);
        let report = artifacts_focused(&source, &candidate, 1, "App launcher");
        assert_eq!(report.status, Status::PreparationRequired);
        assert!(
            report
                .diagnostic
                .as_deref()
                .is_some_and(|value| value.contains("0 candidate"))
        );
    }

    #[test]
    fn comparison_core_stays_fast() {
        let source = artifact(vec![state("base", NodeEvidence::default())]);
        let candidate = source.clone();
        let started = Instant::now();
        for _ in 0..1_000 {
            assert_eq!(artifacts(&source, &candidate, 0).status, Status::Pass);
        }
        assert!(started.elapsed().as_secs_f64() < 1.0);
    }
}
