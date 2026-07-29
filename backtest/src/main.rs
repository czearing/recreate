use anyhow::Context;
use clap::Parser;
use recreate_backtest::{
    browser,
    capture,
    cli::{Cli, Command},
    compare,
    deadline::Deadline,
    fixture,
    model::{Report, Status, Viewport},
    pipeline, report,
};
use std::{fs, time::Instant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let process_started = Instant::now();
    let cli = Cli::parse();
    match cli.command {
        Command::Prepare(args) => {
            let endpoint = args
                .cdp_url
                .unwrap_or_else(|| browser::endpoint(args.side).into());
            let (session, process) = capture::prepare_session(
                args.side.as_str(),
                &args.url,
                &endpoint,
                Viewport {
                    width: args.width,
                    height: args.height,
                },
                args.ready_selector.as_deref(),
            )
            .await?;
            if !args.non_interactive {
                eprintln!("Complete access in the visible browser, then press Enter.");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
            }
            capture::write_session(&args.session, &session)?;
            println!("{}", serde_json::to_string_pretty(&session)?);
            std::mem::forget(process);
        }
        Command::Record(args) => {
            let session = capture::read_session(&args.session)?;
            let artifact = capture::record(&session, capture::parse_viewports(&args.viewports)?).await?;
            capture::write_artifact(&args.out, &artifact)?;
            let evidence = args.out.with_extension("evidence");
            capture::save_screenshots(&artifact, &evidence)?;
            println!(
                "{}",
                serde_json::json!({
                    "artifact":args.out,
                    "digest":artifact.digest,
                    "states":artifact.states.len()
                })
            );
        }
        Command::Compare(args) => {
            let report = execute_compare(&args, process_started).await?;
            print!("{}", report::text(&report));
            if report.status == Status::Fail {
                std::process::exit(1);
            }
            if report.status != Status::Pass {
                std::process::exit(2);
            }
        }
        Command::Pipeline(args) => {
            let mut command_args = args.recreate_args;
            if command_args.is_empty() {
                command_args = vec![
                    "capture".into(),
                    args.source_url.clone(),
                    "--out".into(),
                    args.work_dir.join("capture").display().to_string(),
                    "--baseline-only".into(),
                ];
            }
            let process = pipeline::run(&args.recreate_bin, &command_args, &args.work_dir)?;
            println!(
                "{}",
                serde_json::json!({
                    "recreate_exit":process.exit_code,
                    "recreate_ms":process.elapsed_ms,
                    "candidate_url":args.candidate_url
                })
            );
        }
        Command::Benchmark(args) => {
            let artifact = capture::read_artifact(&args.artifact)?;
            let session = capture::read_session(&args.candidate_session)?;
            let mut durations = Vec::new();
            for _ in 0..args.iterations {
                let started = Instant::now();
                let deadline = Deadline::new(args.budget_ms.min(4800));
                let actual = capture::compare_candidate(deadline, &artifact, &session).await?;
                let report = compare::artifact(&artifact, &actual, started.elapsed().as_millis());
                anyhow::ensure!(report.status != Status::Inconclusive, "benchmark inconclusive");
                durations.push(started.elapsed().as_millis());
            }
            durations.sort_unstable();
            let percentile = |value: f64| {
                let index = ((durations.len().saturating_sub(1)) as f64 * value).ceil() as usize;
                durations[index]
            };
            let evidence = serde_json::json!({
                "iterations":durations.len(),
                "p50_ms":percentile(0.50),
                "p95_ms":percentile(0.95),
                "p99_ms":percentile(0.99),
                "max_ms":durations.last().copied().unwrap_or_default(),
                "durations_ms":durations
            });
            println!("{}", serde_json::to_string_pretty(&evidence)?);
            anyhow::ensure!(percentile(0.95) <= 4000, "p95 exceeds 4000ms");
            anyhow::ensure!(percentile(0.99) <= 4500, "p99 exceeds 4500ms");
            anyhow::ensure!(
                durations.last().copied().unwrap_or_default() < 5000,
                "maximum exceeds five seconds"
            );
        }
        Command::Qualify(args) => {
            fixture::qualify(&args.fixtures, args.out.as_deref(), args.repeat).await?;
        }
    }
    Ok(())
}

async fn execute_compare(
    args: &recreate_backtest::cli::CompareArgs,
    process_started: Instant,
) -> anyhow::Result<Report> {
    let artifact = capture::read_artifact(&args.artifact)?;
    let session = capture::read_session(&args.candidate_session)?;
    if session.browser != artifact.source.browser {
        let report = Report {
            status: Status::PreparationRequired,
            elapsed_ms: process_started.elapsed().as_millis(),
            findings: Vec::new(),
            coverage: vec!["browser mismatch".into()],
        };
        report::write(&report, &args.out)?;
        return Ok(report);
    }
    let deadline = Deadline::new(args.budget_ms.min(4800));
    let result = capture::compare_candidate(deadline, &artifact, &session).await;
    let report = match result {
        Ok(actual) => compare::artifact(&artifact, &actual, process_started.elapsed().as_millis()),
        Err(error) => Report {
            status: Status::Inconclusive,
            elapsed_ms: process_started.elapsed().as_millis(),
            findings: Vec::new(),
            coverage: vec![format!("{error:#}")],
        },
    };
    report::write(&report, &args.out)
        .with_context(|| format!("write {}", args.out.display()))?;
    anyhow::ensure!(
        process_started.elapsed().as_millis() < 5000,
        "compare process exceeded five seconds"
    );
    Ok(report)
}

