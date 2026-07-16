use crate::{
    browser, capture,
    cli::{CaptureArgs, VerifyArgs},
    lifecycle_script,
    model::{Node, PageState, Specification},
};
use anyhow::{Context, Result};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::PathBuf};

#[derive(Serialize)]
struct Report {
    passed: bool,
    matched: usize,
    missing: usize,
    text_mismatches: usize,
    geometry_mismatches: usize,
    style_mismatches: usize,
    details: Vec<String>,
}

pub async fn run(args: VerifyArgs) -> Result<()> {
    let specification: Specification = serde_json::from_slice(&fs::read(&args.spec)?)?;
    let (states, trigger) = if let Some(index) = args.interaction {
        let interaction = specification
            .interactions
            .get(index.saturating_sub(1))
            .with_context(|| format!("interaction {index} not found"))?;
        (&interaction.states, Some(interaction.trigger_path.as_str()))
    } else {
        (&specification.states, None)
    };
    let mut totals = Report {
        passed: true,
        matched: 0,
        missing: 0,
        text_mismatches: 0,
        geometry_mismatches: 0,
        style_mismatches: 0,
        details: Vec::new(),
    };
    for expected in states {
        let capture_args = CaptureArgs {
            url: Some(args.url.clone()),
            reuse: false,
            target: None,
            cdp_url: args.cdp_url.clone(),
            out: PathBuf::new(),
            viewports: String::new(),
        };
        let (_, mut cdp) = browser::target(&capture_args).await?;
        cdp.enable(&["Page", "Runtime", "Network", "DOM", "CSS"])
            .await?;
        cdp.send(
            "Page.addScriptToEvaluateOnNewDocument",
            serde_json::json!({ "source": lifecycle_script::SOURCE }),
        )
        .await?;
        let actual = if let Some(trigger) = trigger {
            let _ = capture::capture_state(&mut cdp, expected.viewport.clone(), true).await?;
            click(&mut cdp, trigger).await?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            capture::read_state(&mut cdp, expected.viewport.clone()).await?
        } else {
            capture::capture_state(&mut cdp, expected.viewport.clone(), true).await?
        };
        merge(&mut totals, compare(expected, &actual));
    }
    totals.passed = totals.missing == 0
        && totals.text_mismatches == 0
        && totals.geometry_mismatches == 0
        && totals.style_mismatches == 0;
    println!("{}", serde_json::to_string_pretty(&totals)?);
    if !totals.passed {
        anyhow::bail!("generated page does not match captured evidence");
    }
    Ok(())
}

async fn click(cdp: &mut crate::cdp::Cdp, path: &str) -> Result<()> {
    let expression = format!(
        "document.querySelector({})?.click()",
        serde_json::to_string(path)?
    );
    cdp.evaluate(&expression).await?;
    Ok(())
}

fn compare(expected: &PageState, actual: &PageState) -> Report {
    let actual: BTreeMap<_, _> = actual.nodes.iter().map(|node| (&node.path, node)).collect();
    let mut report = Report {
        passed: true,
        matched: 0,
        missing: 0,
        text_mismatches: 0,
        geometry_mismatches: 0,
        style_mismatches: 0,
        details: Vec::new(),
    };
    for node in &expected.nodes {
        let Some(candidate) = actual.get(&node.path) else {
            report.missing += 1;
            detail(&mut report, format!("missing {}", node.path));
            continue;
        };
        report.matched += 1;
        if node.text != candidate.text {
            report.text_mismatches += 1;
            detail(&mut report, format!("text {}", node.path));
        }
        if !same_rect(node, candidate) {
            report.geometry_mismatches += 1;
            detail(
                &mut report,
                format!(
                    "rect {} expected={:?} actual={:?}",
                    node.path, node.rect, candidate.rect
                ),
            );
        }
        if !same_style(node, candidate) {
            report.style_mismatches += 1;
            detail(&mut report, format!("style {}", node.path));
        }
    }
    report
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

fn merge(total: &mut Report, value: Report) {
    total.matched += value.matched;
    total.missing += value.missing;
    total.text_mismatches += value.text_mismatches;
    total.geometry_mismatches += value.geometry_mismatches;
    total.style_mismatches += value.style_mismatches;
    for value in value.details {
        detail(total, value);
    }
}

fn detail(report: &mut Report, value: String) {
    if report.details.len() < 30 {
        report.details.push(value);
    }
}
