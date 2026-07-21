use crate::{browser, cdp::Cdp, cli::FidelityArgs, fidelity_responsive_script};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsiveFrame {
    #[serde(default)]
    pub width: u32,
    pub identity_stable: bool,
    pub ancestors: Vec<ResponsiveNode>,
    pub flow: Vec<FlowNode>,
    pub text: Vec<TextMetric>,
    pub document: [f64; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResponsiveNode {
    pub depth: usize,
    pub tag: String,
    pub rect: [f64; 4],
    pub style: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextMetric {
    pub path: String,
    pub lines: usize,
    pub widths: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowNode {
    pub owner_depth: usize,
    pub offset: usize,
    pub path: String,
    pub tag: String,
    pub class_name: String,
    pub text: String,
    pub rect: [f64; 4],
    pub style: BTreeMap<String, String>,
}

pub async fn text_map(args: &FidelityArgs, target_id: &str) -> Result<serde_json::Value> {
    let target = browser::list(&args.cdp_url)
        .await?
        .into_iter()
        .find(|value| value.id == target_id)
        .with_context(|| format!("target not found: {target_id}"))?;
    let mut cdp = Cdp::connect(&target.websocket_url).await?;
    cdp.enable(&["Page", "Runtime"]).await?;
    cdp.send("Page.bringToFront", serde_json::json!({})).await?;
    cdp.evaluate(fidelity_responsive_script::TEXT_MAP).await
}

pub async fn trace(
    args: &FidelityArgs,
    target_id: &str,
    text_lock: &serde_json::Value,
) -> Result<Vec<ResponsiveFrame>> {
    let target = browser::list(&args.cdp_url)
        .await?
        .into_iter()
        .find(|value| value.id == target_id)
        .with_context(|| format!("target not found: {target_id}"))?;
    let mut cdp = Cdp::connect(&target.websocket_url).await?;
    cdp.enable(&["Page", "Runtime"]).await?;
    cdp.send("Page.bringToFront", serde_json::json!({})).await?;
    let widths = args
        .widths
        .split(',')
        .map(|value| value.trim().parse::<u32>())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut frames = Vec::new();
    for (index, width) in widths.into_iter().enumerate() {
        browser::set_viewport(&mut cdp, width, args.height).await?;
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let value = cdp
            .evaluate(&fidelity_responsive_script::snapshot(
                &args.label,
                index == 0,
                text_lock,
            ))
            .await?;
        let mut frame: ResponsiveFrame = serde_json::from_value(value)
            .with_context(|| format!("responsive target missing at {width}px on {target_id}"))?;
        frame.width = width;
        frames.push(frame);
    }
    Ok(frames)
}

pub fn compare(source: &[ResponsiveFrame], candidate: &[ResponsiveFrame]) -> Vec<String> {
    let mut details = Vec::new();
    for expected in source {
        let Some(actual) = candidate.iter().find(|frame| frame.width == expected.width) else {
            details.push(format!("responsive: missing {}px", expected.width));
            continue;
        };
        if expected.identity_stable != actual.identity_stable {
            details.push(format!(
                "responsive: DOM identity {}px source={} candidate={}",
                expected.width, expected.identity_stable, actual.identity_stable
            ));
        }
        compare_nodes(expected, actual, &mut details);
        compare_text(expected, actual, &mut details);
        details.truncate(100);
    }
    details
}

fn compare_nodes(expected: &ResponsiveFrame, actual: &ResponsiveFrame, details: &mut Vec<String>) {
    let root_delta = expected
        .ancestors
        .first()
        .zip(actual.ancestors.first())
        .map(|(left, right)| [right.rect[0] - left.rect[0], right.rect[1] - left.rect[1]])
        .unwrap_or_default();
    for node in expected.ancestors.iter().take(2) {
        let Some(candidate) = actual.ancestors.get(node.depth) else {
            details.push(format!(
                "responsive: missing ancestor {} at {}px",
                node.depth, expected.width
            ));
            continue;
        };
        let raw_position_delta = [
            candidate.rect[0] - node.rect[0],
            candidate.rect[1] - node.rect[1],
        ];
        let position_delta = raw_position_delta
            .into_iter()
            .zip(root_delta)
            .map(|(raw, root)| {
                if (raw - root).abs() <= 2.0 {
                    0.0
                } else {
                    raw.abs()
                }
            })
            .fold(0.0, f64::max);
        let size_delta = node.rect[2..]
            .iter()
            .zip(&candidate.rect[2..])
            .map(|(left, right)| (left - right).abs())
            .fold(0.0, f64::max);
        let delta = position_delta.max(size_delta);
        if delta > 2.0 {
            details.push(format!(
                "responsive: ancestor {} geometry {}px delta={delta:.2}",
                node.depth, expected.width
            ));
        }
        for (property, value) in &node.style {
            if candidate.style.get(property) != Some(value) {
                details.push(format!(
                    "responsive: ancestor {} {property} {}px",
                    node.depth, expected.width
                ));
            }
        }
    }
}

fn compare_text(expected: &ResponsiveFrame, actual: &ResponsiveFrame, details: &mut Vec<String>) {
    let actual: BTreeMap<_, _> = actual
        .text
        .iter()
        .map(|metric| (metric.path.as_str(), metric))
        .collect();
    for metric in &expected.text {
        if actual
            .get(metric.path.as_str())
            .is_none_or(|candidate| candidate.lines != metric.lines)
        {
            details.push(format!(
                "responsive: text lines {} at {}px",
                metric.path, expected.width
            ));
        }
    }
}
