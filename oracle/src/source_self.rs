use crate::{
    browser::Browser,
    checkpoint, compare, engine,
    model::{Artifact, Checkpoint, Coverage, Environment, Obligation, Scenario},
};
use std::path::Path;

pub async fn verify(
    browser: &mut Browser,
    source: &str,
    scenarios: &[Scenario],
    environment: &Environment,
    obligations: &[Obligation],
    checkpoints: &[Checkpoint],
    output: &Path,
) -> anyhow::Result<()> {
    let expected = Artifact {
        format: "recreate-oracle/v1".into(),
        source: source.into(),
        environment: environment.clone(),
        scenarios: scenarios.to_vec(),
        obligations: obligations.to_vec(),
        checkpoints: checkpoints.to_vec(),
        coverage: Coverage {
            widths_required: engine::responsive_width_count(scenarios),
            widths_observed: checkpoints
                .iter()
                .filter(|item| item.scenario.starts_with("responsive-"))
                .count(),
            domains_required: checkpoint::DOMAINS
                .iter()
                .map(ToString::to_string)
                .collect(),
            incomplete: Vec::new(),
        },
        payload_digest: String::new(),
    };
    for repetition in 2..=3 {
        let actual = engine::collect(browser, source, scenarios).await?;
        let report = compare::artifacts(&expected, &actual, Default::default());
        if report.certified {
            continue;
        }
        let report_path = output.with_extension(format!("instability-{repetition}.json"));
        std::fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;
        let first = report
            .first_difference
            .as_ref()
            .map(|difference| {
                format!(
                    "{} {} {}",
                    difference.domain, difference.path, difference.scenario
                )
            })
            .unwrap_or_else(|| "unknown difference".into());
        anyhow::bail!(
            "source-self run {repetition} is unstable at {first}; report: {}",
            report_path.display()
        );
    }
    Ok(())
}
