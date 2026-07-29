use crate::{artifact, cli::CompareArgs, collector::collect, compare};
use std::{fs, time::Instant};

pub(crate) async fn compare(args: CompareArgs) -> anyhow::Result<()> {
    let mut expected = if args.diagnostic {
        artifact::read_diagnostic(&args.artifact)?
    } else {
        artifact::read(&args.artifact)?
    };
    if args.interactions_only {
        retain_interactions(&mut expected);
    }
    let mut browser = crate::browser_factory::start(&args.browser).await?;
    browser.prepare().await?;
    anyhow::ensure!(
        expected.environment == browser.environment().await?,
        "browser environment differs from source artifact"
    );
    let started = Instant::now();
    let (actual, early) = if args.fail_fast {
        collect_until_difference(&mut browser, &args.candidate, &expected, started).await?
    } else {
        (
            collect(&mut browser, &args.candidate, &expected.scenarios).await?,
            None,
        )
    };
    browser.close().await;
    let report = early.unwrap_or_else(|| compare::artifacts(&expected, &actual, started.elapsed()));
    let encoded = serde_json::to_vec_pretty(&report)?;
    if let Some(path) = args.out {
        fs::write(path, &encoded)?;
    }
    println!("{}", String::from_utf8(encoded)?);
    anyhow::ensure!(report.certified, "candidate is not certified");
    Ok(())
}

fn retain_interactions(expected: &mut crate::model::Artifact) {
    expected.scenarios.retain(|scenario| {
        scenario.steps.iter().any(|step| {
            matches!(
                step,
                crate::model::Step::Activate { .. }
                    | crate::model::Step::Hover { .. }
                    | crate::model::Step::Key { .. }
            )
        })
    });
    let ids = expected
        .scenarios
        .iter()
        .map(|scenario| scenario.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    expected
        .checkpoints
        .retain(|checkpoint| ids.contains(checkpoint.scenario.as_str()));
}

async fn collect_until_difference(
    browser: &mut crate::browser::Browser,
    candidate: &str,
    expected: &crate::model::Artifact,
    started: Instant,
) -> anyhow::Result<(Vec<crate::model::Checkpoint>, Option<crate::report::Report>)> {
    let mut actual = Vec::new();
    for scenario in &expected.scenarios {
        actual.extend(collect(browser, candidate, std::slice::from_ref(scenario)).await?);
        let mut subset = expected.clone();
        subset.scenarios = vec![scenario.clone()];
        subset
            .checkpoints
            .retain(|checkpoint| checkpoint.scenario == scenario.id);
        subset.coverage.incomplete.clear();
        let report = compare::artifacts(&subset, &actual, started.elapsed());
        if !report.differences.is_empty() {
            return Ok((actual, Some(report)));
        }
    }
    Ok((actual, None))
}

pub(crate) fn has_ambiguous_nodes(checkpoint: &crate::model::Checkpoint) -> bool {
    checkpoint.domains["structure"].value["ambiguous"]
        .as_array()
        .is_some_and(|items| !items.is_empty())
}

pub(crate) fn has_network_evidence(checkpoint: &crate::model::Checkpoint) -> bool {
    let asynchronous = &checkpoint.domains["async"].value;
    asynchronous["network"]
        .as_array()
        .is_some_and(|entries| !entries.is_empty())
        || asynchronous["resources"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty())
        || asynchronous["documentState"]["network"].is_string()
}

pub(crate) fn has_unavailable_network_body(checkpoint: &crate::model::Checkpoint) -> bool {
    checkpoint.domains["async"].value["network"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|entry| entry["body_unavailable"] == true)
}
