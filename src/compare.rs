use crate::{
    browser,
    cli::{CaptureArgs, VerifyArgs},
    compare_capture, compare_node,
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
    let mut bytes = fs::read(&args.spec)?;
    let specification: Specification = simd_json::serde::from_slice(&mut bytes)?;
    let (states, trigger) = if let Some(index) = args.interaction {
        let interaction = specification
            .interactions
            .get(index.saturating_sub(1))
            .with_context(|| format!("interaction {index} not found"))?;
        (&interaction.states, Some(interaction))
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
    let shared_assets = specification
        .states
        .iter()
        .flat_map(|state| state.asset_data.iter())
        .map(|(url, data)| (url.clone(), data.clone()))
        .collect();
    for expected in states {
        let capture_args = CaptureArgs {
            url: Some(args.url.clone()),
            reuse: false,
            reload: false,
            baseline_only: false,
            spec_only: false,
            target: None,
            cdp_url: args.cdp_url.clone(),
            out: PathBuf::new(),
            viewports: String::new(),
        };
        let (target, mut cdp) = browser::target(&capture_args).await?;
        let result = compare_capture::state(&mut cdp, expected, trigger).await;
        drop(cdp);
        let close = recreate_browser::close(&args.cdp_url, &target.id).await;
        let actual = result?;
        close?;
        let animation_state = specification
            .states
            .iter()
            .find(|state| state.viewport == expected.viewport)
            .unwrap_or(expected);
        let mut report = compare_node::compare_with_animation_assets(
            expected,
            &actual,
            animation_state,
            &shared_assets,
        );
        for detail in &mut report.details {
            *detail = format!(
                "{}x{} {detail}",
                expected.viewport.width, expected.viewport.height
            );
        }
        merge(&mut totals, report);
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
    std::mem::forget(specification);
    std::mem::forget(bytes);
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
