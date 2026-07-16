use crate::{
    browser, capture,
    cli::{CaptureArgs, VerifyArgs},
    compare_node::compare,
    lifecycle_script,
    model::Specification,
};
use anyhow::{Context, Result};
use serde::Serialize;
use std::{fs, path::PathBuf};

#[derive(Serialize)]
pub(crate) struct Report {
    pub(crate) passed: bool,
    pub(crate) expected: usize,
    pub(crate) actual: usize,
    pub(crate) matched: usize,
    pub(crate) missing: usize,
    pub(crate) unexpected: usize,
    pub(crate) structure_mismatches: usize,
    pub(crate) attribute_mismatches: usize,
    pub(crate) pseudo_mismatches: usize,
    pub(crate) text_mismatches: usize,
    pub(crate) geometry_mismatches: usize,
    pub(crate) style_mismatches: usize,
    pub(crate) details: Vec<String>,
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
        expected: 0,
        actual: 0,
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
        && totals.unexpected == 0
        && totals.structure_mismatches == 0
        && totals.attribute_mismatches == 0
        && totals.pseudo_mismatches == 0
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

fn merge(total: &mut Report, value: Report) {
    total.expected += value.expected;
    total.actual += value.actual;
    total.matched += value.matched;
    total.missing += value.missing;
    total.unexpected += value.unexpected;
    total.structure_mismatches += value.structure_mismatches;
    total.attribute_mismatches += value.attribute_mismatches;
    total.pseudo_mismatches += value.pseudo_mismatches;
    total.text_mismatches += value.text_mismatches;
    total.geometry_mismatches += value.geometry_mismatches;
    total.style_mismatches += value.style_mismatches;
    for value in value.details {
        detail(total, value);
    }
}

pub(crate) fn detail(report: &mut Report, value: String) {
    if report.details.len() < 30 {
        report.details.push(value);
    }
}
