use crate::{
    artifact, checkpoint,
    cli::{Cli, Command, CompareArgs, RecordArgs},
    compare, discovery,
    model::{Artifact, Coverage, ObligationStatus},
    qualification, scenario, source_self,
};
use std::{fs, time::Instant};

pub use crate::collector::collect;

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Record(args) => record(args).await,
        Command::Compare(args) => compare_command(args).await,
        Command::Qualify(args) => qualification::run(args).await,
        Command::Benchmark(args) => crate::benchmark::run(args).await,
    }
}

pub async fn record(args: RecordArgs) -> anyhow::Result<()> {
    let mut scenarios = scenario::responsive(&args.widths, args.browser.height)?;
    let mut browser = crate::browser_factory::start(&args.browser).await?;
    browser.prepare().await?;
    let environment = browser.environment().await?;
    let trace_width = args.widths.iter().copied().min().unwrap_or(1280);
    let discovered = discovery::run(
        &mut browser,
        &args.source,
        (trace_width, args.browser.height),
    )
    .await?;
    insert_before_async(&mut scenarios, discovered.scenarios);
    let checkpoints = collect(&mut browser, &args.source, &scenarios).await?;
    let clean = checkpoints
        .iter()
        .find(|checkpoint| {
            checkpoint.scenario == "responsive-ascending"
                && checkpoint.viewport.width == trace_width
        })
        .ok_or_else(|| anyhow::anyhow!("trace qualification checkpoint is missing"))?;
    anyhow::ensure!(
        trace_matches_clean(&discovered.traced_checkpoint, clean),
        "discovery instrumentation perturbed clean browser output"
    );
    source_self::verify(
        &mut browser,
        &args.source,
        &scenarios,
        &environment,
        &discovered.obligations,
        &checkpoints,
        &args.out,
    )
    .await?;
    browser.close().await;
    let widths_observed = checkpoints
        .iter()
        .filter(|item| item.scenario.starts_with("responsive-"))
        .count();
    let widths_required = responsive_width_count(&scenarios);
    let mut incomplete = discovered
        .obligations
        .iter()
        .filter(|item| {
            !matches!(
                item.status,
                ObligationStatus::Qualified | ObligationStatus::UnreachableProven
            )
        })
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    if checkpoints.iter().any(has_ambiguous_nodes) {
        incomplete.push("ambiguous-node-assignment".into());
    }
    if discovered
        .obligations
        .iter()
        .any(|obligation| obligation.kind == "fetch")
        && !checkpoints.iter().any(has_network_evidence)
    {
        incomplete.push("network-registration-without-response-fixture".into());
    }
    if checkpoints.iter().any(has_unavailable_network_body) {
        incomplete.push("network-response-body-unavailable".into());
    }
    let artifact = artifact::seal(Artifact {
        format: "recreate-oracle/v1".into(),
        source: args.source,
        environment,
        scenarios,
        obligations: discovered.obligations,
        checkpoints,
        coverage: Coverage {
            widths_required,
            widths_observed,
            domains_required: checkpoint::DOMAINS
                .iter()
                .map(ToString::to_string)
                .collect(),
            incomplete,
        },
        payload_digest: String::new(),
    })?;
    artifact::write(&args.out, &artifact)?;
    println!("{}", serde_json::to_string_pretty(&artifact.coverage)?);
    Ok(())
}

pub fn trace_matches_clean(
    traced: &crate::model::Checkpoint,
    clean: &crate::model::Checkpoint,
) -> bool {
    [
        "interaction",
        "structure",
        "accessibility",
        "motion",
        "geometry",
        "style",
        "compositor",
    ]
    .iter()
    .all(|domain| traced.domains[*domain].digest == clean.domains[*domain].digest)
}

pub(crate) fn responsive_width_count(scenarios: &[crate::model::Scenario]) -> usize {
    scenarios
        .iter()
        .filter(|scenario| scenario.id.starts_with("responsive-"))
        .flat_map(|scenario| &scenario.steps)
        .map(|step| match step {
            crate::model::Step::SetViewport { .. } => 1,
            crate::model::Step::ResizePath { widths, .. } => widths.len(),
            _ => 0,
        })
        .sum()
}

fn insert_before_async(
    scenarios: &mut Vec<crate::model::Scenario>,
    discovered: Vec<crate::model::Scenario>,
) {
    let async_scenario = scenarios
        .pop()
        .filter(|scenario| scenario.id == "async-settled");
    scenarios.extend(discovered);
    scenarios.extend(async_scenario);
}

pub async fn compare_command(args: CompareArgs) -> anyhow::Result<()> {
    let expected = artifact::read(&args.artifact)?;
    let mut browser = crate::browser_factory::start(&args.browser).await?;
    browser.prepare().await?;
    let actual_environment = browser.environment().await?;
    anyhow::ensure!(
        expected.environment == actual_environment,
        "browser environment differs from source artifact"
    );
    let started = Instant::now();
    let actual = collect(&mut browser, &args.candidate, &expected.scenarios).await?;
    browser.close().await;
    let report = compare::artifacts(&expected, &actual, started.elapsed());
    let encoded = serde_json::to_vec_pretty(&report)?;
    if let Some(path) = args.out {
        fs::write(path, &encoded)?;
    }
    println!("{}", String::from_utf8(encoded)?);
    anyhow::ensure!(report.certified, "candidate is not certified");
    Ok(())
}

fn has_ambiguous_nodes(checkpoint: &crate::model::Checkpoint) -> bool {
    checkpoint.domains["structure"].value["ambiguous"]
        .as_array()
        .is_some_and(|items| !items.is_empty())
}

fn has_network_evidence(checkpoint: &crate::model::Checkpoint) -> bool {
    let asynchronous = &checkpoint.domains["async"].value;
    asynchronous["network"]
        .as_array()
        .is_some_and(|entries| !entries.is_empty())
        || asynchronous["resources"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty())
        || asynchronous["documentState"]["network"].is_string()
}

fn has_unavailable_network_body(checkpoint: &crate::model::Checkpoint) -> bool {
    checkpoint.domains["async"].value["network"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|entry| entry["body_unavailable"] == true)
}
