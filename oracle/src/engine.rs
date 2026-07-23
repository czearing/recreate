pub use crate::collector::collect;
use crate::{
    artifact, checkpoint,
    cli::{Cli, Command, RecordArgs},
    discovery, engine_support,
    model::{Artifact, Coverage, ObligationStatus},
    qualification, scenario, source_self,
};
use anyhow::Context;
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Record(args) => record(args).await,
        Command::Compare(args) => engine_support::compare(args).await,
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
    let mut discovered = discovery::run(
        &mut browser,
        &args.source,
        (trace_width, args.browser.height),
        args.diagnostic,
    )
    .await
    .context("recording source discovery graph")?;
    if args.diagnostic {
        discovered.scenarios.retain(|scenario| {
            [
                "interaction-",
                "state-sequence-",
                "keyboard-navigation",
                "successor-",
            ]
            .iter()
            .any(|prefix| scenario.id.starts_with(prefix))
        });
        let retained = discovered
            .scenarios
            .iter()
            .map(|scenario| scenario.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        for obligation in &mut discovered.obligations {
            obligation
                .scenarios
                .retain(|scenario| retained.contains(scenario.as_str()));
            if obligation.scenarios.is_empty()
                && matches!(obligation.status, ObligationStatus::Qualified)
            {
                obligation.status = ObligationStatus::Uncovered;
            }
        }
    }
    insert_before_async(&mut scenarios, discovered.scenarios);
    if args.diagnostic {
        scenarios.retain(|scenario| {
            [
                "interaction-",
                "state-sequence-",
                "keyboard-navigation",
                "successor-",
            ]
            .iter()
            .any(|prefix| scenario.id.starts_with(prefix))
        });
    }
    let checkpoints = collect(&mut browser, &args.source, &scenarios)
        .await
        .context("collecting authoritative source checkpoints")?;
    let mut incomplete = Vec::new();
    let clean = checkpoints
        .iter()
        .find(|checkpoint| {
            checkpoint.scenario == "responsive-ascending"
                && checkpoint.viewport.width == trace_width
        })
        .ok_or_else(|| anyhow::anyhow!("trace qualification checkpoint is missing"))?;
    if let Some((domain, path, traced, clean)) =
        crate::trace_qualification::difference(&discovered.traced_checkpoint, clean)
    {
        if !args.diagnostic {
            anyhow::bail!(
                "discovery instrumentation perturbed {domain} at {path}: \
                 traced={traced} clean={clean}"
            );
        }
        incomplete.push(format!("diagnostic-trace-instability:{domain}:{path}"));
    }
    if args.diagnostic {
        incomplete.push("diagnostic-source-self-unverified".into());
        incomplete.push("diagnostic-discovered-responsive-motion-skipped".into());
    } else {
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
    }
    browser.close().await;
    let widths_observed = checkpoints
        .iter()
        .filter(|item| item.scenario.starts_with("responsive-"))
        .count();
    let widths_required = responsive_width_count(&scenarios);
    incomplete.extend(
        discovered
            .obligations
            .iter()
            .filter(|item| {
                !matches!(
                    item.status,
                    ObligationStatus::Qualified | ObligationStatus::UnreachableProven
                )
            })
            .map(|item| item.id.clone())
            .collect::<Vec<_>>(),
    );
    if checkpoints.iter().any(engine_support::has_ambiguous_nodes) {
        incomplete.push("ambiguous-node-assignment".into());
    }
    if discovered
        .obligations
        .iter()
        .any(|obligation| obligation.kind == "fetch")
        && !checkpoints.iter().any(engine_support::has_network_evidence)
    {
        incomplete.push("network-registration-without-response-fixture".into());
    }
    if checkpoints
        .iter()
        .any(engine_support::has_unavailable_network_body)
    {
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
