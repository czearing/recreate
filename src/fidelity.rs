use crate::{cli::FidelityArgs, fidelity_responsive};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[path = "fidelity/compare.rs"]
mod compare;
#[path = "fidelity/input.rs"]
mod input;
#[path = "fidelity/trace.rs"]
mod trace;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    nodes: Vec<NodeSnapshot>,
    animations: Vec<AnimationSnapshot>,
    document: [f64; 2],
    root_hovered: bool,
    hit_path: Option<String>,
    visibility: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeSnapshot {
    path: String,
    tag: String,
    class_name: String,
    text: String,
    rect: [f64; 4],
    style: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnimationSnapshot {
    target: String,
    pseudo: Option<String>,
    current_time: f64,
    duration: f64,
    delay: f64,
    easing: String,
    properties: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize)]
struct Frame {
    elapsed_ms: u64,
    snapshot: Snapshot,
}

#[derive(Clone, Debug, Serialize)]
struct Trace {
    label: String,
    hover: Vec<Frame>,
    leave: Vec<Frame>,
}

#[derive(Debug, Serialize)]
struct Report {
    passed: bool,
    source: Trace,
    candidate: Trace,
    responsive_source: Vec<fidelity_responsive::ResponsiveFrame>,
    responsive_candidate: Vec<fidelity_responsive::ResponsiveFrame>,
    details: Vec<String>,
}

pub async fn run(args: FidelityArgs) -> Result<()> {
    trace::reset(&args, &args.source_target).await?;
    trace::reset(&args, &args.candidate_target).await?;
    let source = trace::capture(&args, &args.source_target).await?;
    let candidate = trace::capture(&args, &args.candidate_target).await?;
    let text_lock = fidelity_responsive::text_map(&args, &args.source_target).await?;
    let responsive_source =
        fidelity_responsive::trace(&args, &args.source_target, &text_lock).await?;
    let responsive_candidate =
        fidelity_responsive::trace(&args, &args.candidate_target, &text_lock).await?;
    let mut details = compare::traces(&source, &candidate);
    details.extend(fidelity_responsive::compare(
        &responsive_source,
        &responsive_candidate,
    ));
    details.truncate(100);
    let report = Report {
        passed: details.is_empty(),
        source,
        candidate,
        responsive_source,
        responsive_candidate,
        details,
    };
    if let Some(path) = &args.output {
        std::fs::write(path, serde_json::to_vec_pretty(&report)?)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "passed": report.passed,
            "detail_count": report.details.len(),
            "details": report.details.iter().take(20).collect::<Vec<_>>(),
            "source_label": report.source.label,
            "candidate_label": report.candidate.label,
            "output": args.output,
        }))?
    );
    anyhow::ensure!(report.passed, "hover fidelity mismatch");
    Ok(())
}

#[cfg(test)]
fn compare(source: &Trace, candidate: &Trace) -> Vec<String> {
    compare::traces(source, candidate)
}

#[cfg(test)]
#[path = "fidelity_tests.rs"]
mod tests;
