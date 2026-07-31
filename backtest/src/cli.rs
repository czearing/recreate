use crate::{
    blackbox, browser, capture, fixture,
    model::{Artifact, SCHEMA_VERSION, Session, Side, Viewport},
    report,
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Debug, Parser)]
#[command(
    name = "recreate-backtest",
    bin_name = "recreate backtest",
    about = "Independent browser backtesting for Recreate output"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run(Run),
    Prepare(Prepare),
    Record(Record),
    Compare(Compare),
    Benchmark(Benchmark),
    Qualify(Qualify),
    Pipeline(Pipeline),
}

#[derive(Debug, Args)]
struct Run {
    /// Source page URL.
    #[arg(long)]
    source: String,
    /// Recreation URL, including localhost.
    #[arg(long)]
    recreation: String,
    /// Directory for comparison evidence and reports.
    #[arg(long, default_value = "recreate-backtest-output")]
    output: PathBuf,
    /// Case-insensitive visible name or semantic region, such as "toolbar" or "App launcher"; not a CSS selector.
    #[arg(long)]
    focus: Option<String>,
    /// Case-insensitive text of a finding that is expected and must not fail the
    /// comparison, such as a sign-in fixture. Repeatable.
    #[arg(long = "allow")]
    allow: Vec<String>,
    /// Capture the source again instead of reusing a recent capture.
    #[arg(long)]
    recapture: bool,
    #[arg(long, hide = true)]
    source_cdp_url: Option<String>,
    #[arg(long, hide = true)]
    source_target: Option<String>,
    #[arg(long, hide = true)]
    recreation_cdp_url: Option<String>,
    #[arg(long, hide = true)]
    recreation_target: Option<String>,
}

#[derive(Clone, Debug, ValueEnum)]
enum PrepareSide {
    Source,
    Candidate,
}

#[derive(Debug, Args)]
struct Prepare {
    side: PrepareSide,
    #[arg(long)]
    url: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    browser: Option<PathBuf>,
    #[arg(
        long,
        requires = "target",
        conflicts_with_all = ["browser", "headless"]
    )]
    cdp_url: Option<String>,
    #[arg(long, requires = "cdp_url")]
    target: Option<String>,
    #[arg(long, default_value_t = 1440)]
    width: u32,
    #[arg(long, default_value_t = 900)]
    height: u32,
    #[arg(long)]
    headless: bool,
}

#[derive(Debug, Args)]
struct Record {
    #[arg(long)]
    session: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    baseline_only: bool,
}

#[derive(Debug, Args)]
struct Compare {
    #[arg(long)]
    source: PathBuf,
    #[arg(long)]
    candidate: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Case-insensitive visible name or semantic region, such as "toolbar" or "App launcher"; not a CSS selector.
    #[arg(long)]
    focus: Option<String>,
    /// Case-insensitive text of a finding that is expected and must not fail the
    /// comparison, such as a sign-in fixture. Repeatable.
    #[arg(long = "allow")]
    allow: Vec<String>,
}

#[derive(Debug, Args)]
struct Benchmark {
    #[arg(long)]
    source: PathBuf,
    #[arg(long)]
    candidate: PathBuf,
    #[arg(long, default_value_t = 20)]
    repeat: usize,
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct Qualify {
    #[arg(long, default_value = "fixtures")]
    fixtures: PathBuf,
    #[arg(long, default_value = "target/qualification.json")]
    output: PathBuf,
    #[arg(long, default_value_t = 1)]
    repeat: usize,
    #[arg(long)]
    browser: Option<PathBuf>,
    #[arg(long)]
    recreate_bin: Option<PathBuf>,
    #[arg(long)]
    recreate_arg: Vec<String>,
}

#[derive(Debug, Args)]
struct Pipeline {
    #[arg(long)]
    source_url: String,
    #[arg(long)]
    candidate_url: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    browser: Option<PathBuf>,
    #[arg(long)]
    recreate_bin: Option<PathBuf>,
    #[arg(long)]
    recreate_arg: Vec<String>,
}

impl Cli {
    pub async fn run(self) -> anyhow::Result<()> {
        match self.command {
            Command::Run(args) => run(args).await,
            Command::Prepare(args) => prepare(args).await,
            Command::Record(args) => record(args).await,
            Command::Compare(args) => compare(args).await,
            Command::Benchmark(args) => benchmark(args).await,
            Command::Qualify(args) => {
                fixture::qualify(
                    &args.fixtures,
                    Some(&args.output),
                    args.repeat,
                    args.browser.as_deref(),
                    args.recreate_bin.as_deref(),
                    &args.recreate_arg,
                )
                .await?;
                Ok(())
            }
            Command::Pipeline(args) => pipeline(args).await,
        }
    }
}

async fn run(args: Run) -> anyhow::Result<()> {
    fs::create_dir_all(&args.output)?;
    let source_session = args.output.join("source.session.json");
    let source_artifact = args.output.join("source.artifact.json");
    let candidate_session = args.output.join("recreation.session.json");
    let reuse = !args.recapture && reusable_source(&source_artifact, &args.source);
    if reuse {
        println!(
            "reusing the source captured in the last {} minutes; pass --recapture to capture it again",
            SOURCE_REUSE_SECONDS / 60
        );
    } else {
        prepare(Prepare {
            side: PrepareSide::Source,
            url: args.source,
            output: source_session.clone(),
            browser: None,
            cdp_url: args.source_cdp_url,
            target: args.source_target,
            width: 1440,
            height: 900,
            headless: false,
        })
        .await?;
    }
    prepare(Prepare {
        side: PrepareSide::Candidate,
        url: args.recreation,
        output: candidate_session.clone(),
        browser: None,
        cdp_url: args.recreation_cdp_url,
        target: args.recreation_target,
        width: 1440,
        height: 900,
        headless: true,
    })
    .await?;
    if !reuse {
        let source: Session = read_json(&source_session)?;
        let artifact = capture::record_source_snapshot(&source).await?;
        write_json(&source_artifact, &artifact)?;
    }
    compare_snapshot(
        &source_artifact,
        &candidate_session,
        &args.output,
        args.focus,
        &args.allow,
    )
    .await
}

/// A source page does not change while its recreation is being fixed, so
/// re-capturing it on every run is the largest avoidable cost in that loop.
/// The capture is only reused briefly, so a source that does change is noticed.
const SOURCE_REUSE_SECONDS: u64 = 900;

fn reusable_source(artifact: &Path, source_url: &str) -> bool {
    let Ok(metadata) = fs::metadata(artifact) else {
        return false;
    };
    let fresh = metadata
        .modified()
        .ok()
        .and_then(|modified| std::time::SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age.as_secs() < SOURCE_REUSE_SECONDS);
    if !fresh {
        return false;
    }
    read_json::<Artifact>(artifact)
        .is_ok_and(|value| value.verify().is_ok() && value.source.requested_url == source_url)
}

async fn compare_snapshot(
    source: &Path,
    candidate: &Path,
    output: &Path,
    focus: Option<String>,
    allowances: &[String],
) -> anyhow::Result<()> {
    let (cancel_watchdog, watchdog) = mpsc::channel();
    std::thread::spawn(move || {
        if watchdog
            .recv_timeout(Duration::from_millis(crate::deadline::WATCHDOG_MS))
            .is_err()
        {
            eprintln!(
                "recreate-backtest: compare command exceeded {}ms",
                crate::deadline::WATCHDOG_MS
            );
            std::process::exit(124);
        }
    });
    let comparison = output.join("comparison");
    if comparison.exists() {
        fs::remove_dir_all(&comparison)?;
    }
    let artifact: Artifact = read_json(source)?;
    let session: Session = read_json(candidate)?;
    let value = fixture::compare_snapshot(&artifact, &session, focus.as_deref()).await;
    let _ = cancel_watchdog.send(());
    finish_compare(&comparison, &value, allowances)
}

async fn prepare(args: Prepare) -> anyhow::Result<()> {
    if let (Some(endpoint), Some(target_id)) = (&args.cdp_url, &args.target) {
        let target = browser::target(endpoint, target_id).await?;
        anyhow::ensure!(
            !target.url.is_empty(),
            "attached target did not report a rendered URL"
        );
        let mut session = Session {
            schema_version: SCHEMA_VERSION,
            side: match args.side {
                PrepareSide::Source => Side::Source,
                PrepareSide::Candidate => Side::Candidate,
            },
            cdp_url: endpoint.clone(),
            target_id: target.id,
            requested_url: args.url,
            rendered_url: target.url,
            browser: browser::version(endpoint).await?,
            executable: "recreate-managed".into(),
            profile: format!("attached:{endpoint}"),
            viewport: Viewport {
                width: args.width,
                height: args.height,
            },
            digest: String::new(),
        };
        session.seal()?;
        write_json(&args.output, &session)?;
        println!("{}", args.output.display());
        return Ok(());
    }
    let executable = browser::find(args.browser.as_deref())?;
    let profile = args
        .output
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(
            ".recreate-backtest-{}-profile",
            match args.side {
                PrepareSide::Source => "source",
                PrepareSide::Candidate => "candidate",
            }
        ));
    let process = browser::launch(&executable, &profile, !args.headless).await?;
    let browser_name = browser::version(&process.endpoint).await?;
    let target = browser::create(&process.endpoint, &args.url).await?;
    let mut session = Session {
        schema_version: SCHEMA_VERSION,
        side: match args.side {
            PrepareSide::Source => Side::Source,
            PrepareSide::Candidate => Side::Candidate,
        },
        cdp_url: process.endpoint.clone(),
        target_id: target.id,
        requested_url: args.url.clone(),
        rendered_url: args.url,
        browser: browser_name,
        executable: process.executable.display().to_string(),
        profile: process.profile.display().to_string(),
        viewport: Viewport {
            width: args.width,
            height: args.height,
        },
        digest: String::new(),
    };
    session.seal()?;
    write_json(&args.output, &session)?;
    process.persist();
    println!("{}", args.output.display());
    Ok(())
}

async fn record(args: Record) -> anyhow::Result<()> {
    let session: Session = read_json(&args.session)?;
    let artifact = capture::record_source(&session, args.baseline_only).await?;
    write_json(&args.output, &artifact)?;
    println!("{}", args.output.display());
    Ok(())
}

async fn compare(args: Compare) -> anyhow::Result<()> {
    let (cancel_watchdog, watchdog) = mpsc::channel();
    std::thread::spawn(move || {
        if watchdog
            .recv_timeout(Duration::from_millis(crate::deadline::WATCHDOG_MS))
            .is_err()
        {
            eprintln!(
                "recreate-backtest: compare command exceeded {}ms",
                crate::deadline::WATCHDOG_MS
            );
            std::process::exit(124);
        }
    });
    let result = async move {
        let started = Instant::now();
        if args.output.exists() {
            fs::remove_dir_all(&args.output)?;
        }
        let artifact: Artifact = match read_json(&args.source) {
            Ok(value) => value,
            Err(error) => {
                let value = crate::compare::preparation_required_session(
                    String::new(),
                    started.elapsed().as_millis(),
                    format!("source artifact unavailable: {error}"),
                );
                return finish_compare(&args.output, &value, &args.allow);
            }
        };
        let session: Session = match read_json(&args.candidate) {
            Ok(value) => value,
            Err(error) => {
                let value = crate::compare::preparation_required_session(
                    artifact.digest.clone(),
                    started.elapsed().as_millis(),
                    format!("candidate session unavailable: {error}"),
                );
                return finish_compare(&args.output, &value, &args.allow);
            }
        };
        let value = match args.focus.as_deref() {
            Some(focus) => fixture::compare_once_focused(&artifact, &session, focus).await,
            None => fixture::compare_once(&artifact, &session).await,
        };
        finish_compare(&args.output, &value, &args.allow)
    }
    .await;
    let _ = cancel_watchdog.send(());
    result
}

fn finish_compare(
    output: &Path,
    value: &crate::model::Report,
    allowances: &[String],
) -> anyhow::Result<()> {
    let mut value = value.clone();
    report::apply_allowances(&mut value, allowances);
    let value = &value;
    report::write(output, value)?;
    print!("{}", report::text(value));
    match value.status {
        crate::model::Status::Pass => Ok(()),
        crate::model::Status::Fail => anyhow::bail!("comparison failed"),
        crate::model::Status::Inconclusive | crate::model::Status::PreparationRequired => {
            anyhow::bail!("comparison did not produce a conclusive result")
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkOutput {
    repeat: usize,
    durations_ms: Vec<u128>,
    p95_ms: u128,
    p99_ms: u128,
    maximum_ms: u128,
}

async fn benchmark(args: Benchmark) -> anyhow::Result<()> {
    let artifact: Artifact = read_json(&args.source)?;
    let session: Session = read_json(&args.candidate)?;
    let repeat = args.repeat.max(1);
    let mut durations = Vec::new();
    for _ in 0..repeat {
        let report = fixture::compare_once(&artifact, &session).await;
        anyhow::ensure!(
            !matches!(report.status, crate::model::Status::Inconclusive),
            "benchmark comparison was inconclusive"
        );
        durations.push(report.elapsed_ms);
    }
    durations.sort_unstable();
    let output = BenchmarkOutput {
        repeat,
        p95_ms: percentile(&durations, 0.95),
        p99_ms: percentile(&durations, 0.99),
        maximum_ms: durations.last().copied().unwrap_or_default(),
        durations_ms: durations,
    };
    anyhow::ensure!(output.p95_ms <= 4000, "benchmark p95 exceeded 4000ms");
    anyhow::ensure!(output.p99_ms <= 4500, "benchmark p99 exceeded 4500ms");
    anyhow::ensure!(output.maximum_ms < 5000, "benchmark maximum reached 5000ms");
    let bytes = serde_json::to_vec_pretty(&output)?;
    if let Some(path) = args.output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &bytes)?;
    }
    println!("{}", String::from_utf8_lossy(&bytes));
    Ok(())
}

async fn pipeline(args: Pipeline) -> anyhow::Result<()> {
    fs::create_dir_all(&args.output)?;
    if let Some(binary) = &args.recreate_bin {
        let evidence = blackbox::run(binary, &args.recreate_arg)?;
        write_json(&args.output.join("process.json"), &evidence)?;
        anyhow::ensure!(evidence.success, "selected Recreate child process failed");
    }
    let executable = browser::find(args.browser.as_deref())?;
    let source_profile = args.output.join("source-profile");
    let candidate_profile = args.output.join("candidate-profile");
    let source_process = browser::launch(&executable, &source_profile, false).await?;
    let candidate_process = browser::launch(&executable, &candidate_profile, false).await?;
    let (browser_name, candidate_browser) = tokio::try_join!(
        browser::version(&source_process.endpoint),
        browser::version(&candidate_process.endpoint)
    )?;
    anyhow::ensure!(
        candidate_browser == browser_name,
        "pipeline browsers differ"
    );
    let (source_target, candidate_target) = tokio::try_join!(
        browser::create(&source_process.endpoint, "about:blank"),
        browser::create(&candidate_process.endpoint, "about:blank")
    )?;
    let source_session = make_session(
        Side::Source,
        &source_process,
        source_target.id,
        args.source_url,
        &browser_name,
    )?;
    let candidate_session = make_session(
        Side::Candidate,
        &candidate_process,
        candidate_target.id,
        args.candidate_url,
        &browser_name,
    )?;
    let artifact = capture::record_source(&source_session, false).await?;
    let comparison = fixture::compare_once(&artifact, &candidate_session).await;
    write_json(&args.output.join("source.artifact.json"), &artifact)?;
    write_json(&args.output.join("source.session.json"), &source_session)?;
    write_json(
        &args.output.join("candidate.session.json"),
        &candidate_session,
    )?;
    report::write(&args.output, &comparison)?;
    print!("{}", report::text(&comparison));
    anyhow::ensure!(
        comparison.status == crate::model::Status::Pass,
        "pipeline comparison did not pass"
    );
    Ok(())
}

fn make_session(
    side: Side,
    process: &browser::BrowserProcess,
    target_id: String,
    url: String,
    browser_name: &str,
) -> anyhow::Result<Session> {
    let mut session = Session {
        schema_version: SCHEMA_VERSION,
        side,
        cdp_url: process.endpoint.clone(),
        target_id,
        requested_url: url.clone(),
        rendered_url: url,
        browser: browser_name.into(),
        executable: process.executable.display().to_string(),
        profile: process.profile.display().to_string(),
        viewport: Viewport {
            width: 1440,
            height: 900,
        },
        digest: String::new(),
    };
    session.seal()?;
    Ok(session)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn percentile(values: &[u128], percentile: f64) -> u128 {
    if values.is_empty() {
        return 0;
    }

    values[((values.len() - 1) as f64 * percentile).ceil() as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn exposes_required_commands() {
        let command = Cli::command();
        let names = command
            .get_subcommands()
            .map(|value| value.get_name())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "run",
                "prepare",
                "record",
                "compare",
                "benchmark",
                "qualify",
                "pipeline"
            ]
        );
    }

    #[test]
    fn parses_existing_recreate_target_attachment() {
        let cli = Cli::try_parse_from([
            "recreate-backtest",
            "prepare",
            "source",
            "--url",
            "https://example.com/app",
            "--output",
            "source.session.json",
            "--cdp-url",
            "http://127.0.0.1:9223",
            "--target",
            "target-id",
        ])
        .unwrap();
        let Command::Prepare(args) = cli.command else {
            panic!("expected prepare");
        };
        assert_eq!(args.cdp_url.as_deref(), Some("http://127.0.0.1:9223"));
        assert_eq!(args.target.as_deref(), Some("target-id"));
    }

    #[test]
    fn parses_automatic_generated_project_run() {
        let cli = Cli::try_parse_from([
            "recreate",
            "run",
            "--source",
            "https://example.com",
            "--recreation",
            "http://localhost:8080",
            "--source-cdp-url",
            "http://127.0.0.1:9223",
            "--source-target",
            "target-1",
        ])
        .unwrap();
        let Command::Run(args) = cli.command else {
            panic!("expected run");
        };
        assert_eq!(args.source, "https://example.com");
        assert_eq!(args.recreation, "http://localhost:8080");
        assert_eq!(args.output, PathBuf::from("recreate-backtest-output"));
    }

    #[test]
    fn a_missing_or_unreadable_source_capture_is_never_reused() {
        let directory = tempfile::tempdir().unwrap();
        let artifact = directory.path().join("source.artifact.json");
        assert!(!reusable_source(&artifact, "https://example.com"));
        fs::write(&artifact, "not an artifact").unwrap();
        assert!(!reusable_source(&artifact, "https://example.com"));
    }

    #[test]
    fn recapture_is_available_on_the_run_command() {
        let cli = Cli::try_parse_from([
            "recreate",
            "run",
            "--source",
            "https://example.com",
            "--recreation",
            "http://localhost:8080",
            "--recapture",
            "--allow",
            "signed in as",
        ])
        .unwrap();
        let Command::Run(args) = cli.command else {
            panic!("expected run");
        };
        assert!(args.recapture);
        assert_eq!(args.allow, vec!["signed in as".to_string()]);
    }

    #[test]
    fn inconclusive_timeout_report_is_persisted_before_failure() {
        let directory = tempfile::tempdir().unwrap();
        let report = crate::compare::inconclusive(
            "source-digest".into(),
            crate::deadline::COMPARISON_DEADLINE_MS as u128,
            "comparison deadline expired".into(),
        );
        assert!(finish_compare(directory.path(), &report, &[]).is_err());
        let text = fs::read_to_string(directory.path().join("comparison.txt")).unwrap();
        assert!(text.starts_with("INCONCLUSIVE"));
        assert!(text.contains("comparison deadline expired"));
        let json = fs::read_to_string(directory.path().join("comparison.json")).unwrap();
        assert!(json.contains("\"status\": \"INCONCLUSIVE\""));
    }
}
