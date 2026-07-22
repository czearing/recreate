use crate::{
    checkpoint::DOMAINS,
    digest,
    model::{Artifact, ObligationStatus},
};
use anyhow::Context;
use std::{fs, path::Path};

pub fn seal(mut artifact: Artifact) -> anyhow::Result<Artifact> {
    artifact.payload_digest.clear();
    artifact.payload_digest = digest::json(&artifact)?;
    Ok(artifact)
}

pub fn verify(artifact: &Artifact) -> anyhow::Result<()> {
    verify_with(artifact, false)
}

fn verify_with(artifact: &Artifact, allow_incomplete: bool) -> anyhow::Result<()> {
    let expected = artifact.payload_digest.clone();
    let actual = seal(artifact.clone())?.payload_digest;
    anyhow::ensure!(expected == actual, "artifact digest mismatch");
    anyhow::ensure!(
        artifact.format == "recreate-oracle/v1",
        "unsupported artifact format"
    );
    anyhow::ensure!(!artifact.source.is_empty(), "artifact source is empty");
    anyhow::ensure!(
        artifact.coverage.widths_required == artifact.coverage.widths_observed,
        "responsive width coverage is incomplete"
    );
    let observed_widths = artifact
        .checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.scenario.starts_with("responsive-"))
        .count();
    anyhow::ensure!(
        observed_widths == artifact.coverage.widths_observed,
        "responsive checkpoint count does not match coverage"
    );
    anyhow::ensure!(
        artifact.coverage.domains_required
            == DOMAINS.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "artifact domain contract is incomplete"
    );
    let scenario_ids = artifact
        .scenarios
        .iter()
        .map(|scenario| scenario.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    anyhow::ensure!(
        scenario_ids.len() == artifact.scenarios.len(),
        "scenario ids are not unique"
    );
    for scenario in &artifact.scenarios {
        let expected = scenario
            .steps
            .iter()
            .map(|step| match step {
                crate::model::Step::ResizePath { widths, .. } => widths.len(),
                crate::model::Step::Reset | crate::model::Step::Hover { .. } => 0,
                _ => 1,
            })
            .sum::<usize>();
        let actual = artifact
            .checkpoints
            .iter()
            .filter(|checkpoint| checkpoint.scenario == scenario.id)
            .count();
        anyhow::ensure!(
            expected == actual,
            "scenario checkpoint coverage mismatch: {}",
            scenario.id
        );
    }
    for obligation in &artifact.obligations {
        if !allow_incomplete {
            anyhow::ensure!(
                matches!(
                    obligation.status,
                    ObligationStatus::Qualified | ObligationStatus::UnreachableProven
                ),
                "unqualified obligation: {}",
                obligation.id
            );
        }
        anyhow::ensure!(
            obligation
                .scenarios
                .iter()
                .all(|scenario| scenario_ids.contains(scenario.as_str())),
            "obligation references an unknown scenario: {}",
            obligation.id
        );
    }
    for checkpoint in &artifact.checkpoints {
        for name in DOMAINS {
            let domain = checkpoint
                .domains
                .get(*name)
                .with_context(|| format!("checkpoint missing domain {name}"))?;
            anyhow::ensure!(
                domain.digest == digest::json(&domain.value)?,
                "checkpoint domain digest mismatch: {name}"
            );
        }
    }
    if !allow_incomplete {
        anyhow::ensure!(
            artifact.coverage.incomplete.is_empty(),
            "artifact coverage is incomplete: {}",
            artifact.coverage.incomplete.join(", ")
        );
    }
    Ok(())
}

pub fn read(path: &Path) -> anyhow::Result<Artifact> {
    read_with(path, false)
}

pub fn read_diagnostic(path: &Path) -> anyhow::Result<Artifact> {
    read_with(path, true)
}

fn read_with(path: &Path, allow_incomplete: bool) -> anyhow::Result<Artifact> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let artifact = serde_json::from_slice(&bytes).context("parse oracle artifact")?;
    verify_with(&artifact, allow_incomplete)?;
    Ok(artifact)
}

pub fn write(path: &Path, artifact: &Artifact) -> anyhow::Result<()> {
    let parent = path.parent().context("artifact path has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(artifact)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Coverage, Environment};

    #[test]
    fn tampering_is_rejected() {
        let mut artifact = seal(Artifact {
            format: "recreate-oracle/v1".into(),
            source: "a".into(),
            environment: Environment {
                schema: 1,
                browser_product: "test".into(),
                browser_revision: "1".into(),
                protocol_version: "1".into(),
                command_line_digest: "flags".into(),
                operating_system: "test".into(),
                architecture: "test".into(),
                locale: "en-US".into(),
                timezone: "UTC".into(),
                color_scheme: "light".into(),
                reduced_motion: false,
                device_scale_factor_milli: 1000,
            },
            scenarios: vec![],
            obligations: vec![],
            checkpoints: vec![],
            coverage: Coverage {
                widths_required: 0,
                widths_observed: 0,
                domains_required: vec![],
                incomplete: vec![],
            },
            payload_digest: String::new(),
        })
        .unwrap();
        artifact.source = "tampered".into();
        assert!(verify(&artifact).is_err());
    }
}
