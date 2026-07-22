use crate::{
    artifact, checkpoint,
    cli::QualifyArgs,
    compare, discovery, engine, holdout,
    model::{Artifact, Coverage},
    qualification_server, scenario,
};
use serde::Serialize;
use std::{collections::BTreeMap, fs, time::Instant};

#[derive(Serialize)]
struct Evidence {
    source_self: bool,
    equivalent: bool,
    mutants_killed: usize,
    mutants_total: usize,
    localized_mutants: usize,
    kills_by_domain: BTreeMap<String, usize>,
    hidden_killed: usize,
    hidden_total: usize,
    network_responses: usize,
    p50_ms: u128,
    p95_ms: u128,
    p99_ms: u128,
}

pub async fn run(args: QualifyArgs) -> anyhow::Result<()> {
    let server = qualification_server::Server::start()?;
    let true_url = server.url(true);
    let false_url = server.url(false);
    let network_urls = (true_url.as_str(), false_url.as_str());
    let source = holdout::page_url(&server, &args.fixtures.join("source.html"), network_urls)?;
    let equivalent = holdout::page_url(
        &server,
        &args.fixtures.join("equivalent.html"),
        network_urls,
    )?;
    let mutations = holdout::load(
        &args.fixtures,
        args.holdouts.as_deref(),
        &server,
        network_urls,
    )?;
    let mut scenarios = scenario::responsive(&[320], args.browser.height)?;
    let mut browser = crate::browser_factory::start(&args.browser).await?;
    browser.prepare().await?;
    let environment = browser.environment().await?;
    let discovered = discovery::run(&mut browser, &source, (320, args.browser.height)).await?;
    let async_scenario = scenarios.pop().expect("async scenario");
    scenarios.extend(discovered.scenarios);
    scenarios.push(async_scenario);
    let checkpoints = engine::collect(&mut browser, &source, &scenarios).await?;
    let network_responses = checkpoints
        .iter()
        .filter_map(|checkpoint| checkpoint.domains["async"].value["network"].as_array())
        .map(Vec::len)
        .sum();
    let clean = checkpoints
        .iter()
        .find(|checkpoint| checkpoint.scenario == "responsive-ascending")
        .expect("clean trace checkpoint");
    anyhow::ensure!(
        crate::trace_qualification::matches_clean(&discovered.traced_checkpoint, clean),
        "discovery instrumentation perturbed authoritative output"
    );
    let expected = artifact::seal(Artifact {
        format: "recreate-oracle/v1".into(),
        source: source.clone(),
        environment,
        scenarios: scenarios.clone(),
        obligations: discovered.obligations,
        checkpoints,
        coverage: Coverage {
            widths_required: 8,
            widths_observed: 8,
            domains_required: checkpoint::DOMAINS
                .iter()
                .map(ToString::to_string)
                .collect(),
            incomplete: Vec::new(),
        },
        payload_digest: String::new(),
    })?;
    artifact::verify(&expected)?;
    let self_actual = engine::collect(&mut browser, &source, &scenarios).await?;
    let self_report = compare::artifacts(&expected, &self_actual, Default::default());
    anyhow::ensure!(
        self_report == compare::reference_artifacts(&expected, &self_actual, Default::default()),
        "optimized and reference differs disagree on source-self"
    );
    if !self_report.certified {
        eprintln!(
            "source-self differences: {}",
            serde_json::to_string(&self_report.differences)?
        );
    }
    let source_self = self_report.certified;
    let mut samples = Vec::new();
    let mut equivalent_ok = true;
    for _ in 0..5 {
        let started = Instant::now();
        let actual = engine::collect(&mut browser, &equivalent, &scenarios).await?;
        let elapsed = started.elapsed();
        let report = compare::artifacts(&expected, &actual, elapsed);
        let mut comparable = report.clone();
        comparable.elapsed_ms = 0;
        anyhow::ensure!(
            comparable == compare::reference_artifacts(&expected, &actual, Default::default()),
            "optimized and reference differs disagree on equivalent pair"
        );
        if !report.certified {
            eprintln!(
                "equivalent differences: {}",
                serde_json::to_string(&report.differences)?
            );
        }
        equivalent_ok &= report.certified;
        samples.push(elapsed.as_millis());
    }
    let mut killed = 0;
    let mut localized = 0;
    let mut hidden_killed = 0;
    let mut kills_by_domain = BTreeMap::new();
    for mutation in &mutations {
        match engine::collect(&mut browser, &mutation.url, &scenarios).await {
            Ok(actual) => {
                let report = compare::artifacts(&expected, &actual, Default::default());
                anyhow::ensure!(
                    report == compare::reference_artifacts(&expected, &actual, Default::default()),
                    "optimized and reference differs disagree on mutant"
                );
                if !report.certified {
                    killed += 1;
                    hidden_killed += usize::from(mutation.hidden);
                    if report
                        .first_difference
                        .as_ref()
                        .is_some_and(|difference| difference.domain == mutation.expected_domain)
                    {
                        localized += 1;
                    } else {
                        eprintln!(
                            "mutation localization mismatch expected={} actual={:?}",
                            mutation.expected_domain,
                            report
                                .first_difference
                                .as_ref()
                                .map(|difference| difference.domain.as_str())
                        );
                    }
                    *kills_by_domain
                        .entry(mutation.expected_domain.clone())
                        .or_default() += 1;
                }
            }
            Err(_) => {
                killed += 1;
                hidden_killed += usize::from(mutation.hidden);
            }
        }
    }
    browser.close().await;
    samples.sort_unstable();
    let percentile = |value: usize| samples[(samples.len() * value / 100).min(samples.len() - 1)];
    let evidence = Evidence {
        source_self,
        equivalent: equivalent_ok,
        mutants_killed: killed,
        mutants_total: mutations.len(),
        localized_mutants: localized,
        kills_by_domain,
        hidden_killed,
        hidden_total: mutations.iter().filter(|mutation| mutation.hidden).count(),
        network_responses,
        p50_ms: percentile(50),
        p95_ms: percentile(95),
        p99_ms: percentile(99),
    };
    let encoded = serde_json::to_vec_pretty(&evidence)?;
    if let Some(path) = args.out {
        fs::write(path, &encoded)?;
    }
    println!("{}", String::from_utf8(encoded)?);
    anyhow::ensure!(source_self, "source-self qualification failed");
    anyhow::ensure!(equivalent_ok, "equivalent transform was rejected");
    anyhow::ensure!(killed == mutations.len(), "one or more mutants survived");
    anyhow::ensure!(
        localized == mutations.len(),
        "one or more mutants were localized to the wrong first domain"
    );
    anyhow::ensure!(evidence.p95_ms <= 2_000, "warm p95 exceeded 2000ms");
    anyhow::ensure!(evidence.p99_ms <= 2_500, "warm p99 exceeded 2500ms");
    Ok(())
}
