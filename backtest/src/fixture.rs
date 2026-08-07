use crate::{
    blackbox::{self, ProcessEvidence},
    browser, capture, compare,
    deadline::{COMPARISON_DEADLINE_MS, Deadline},
    model::{Artifact, Report, SCHEMA_VERSION, Session, Side, Status, Viewport},
    report,
    server::Server,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, time::Instant};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedCase {
    schema_version: u32,
    case: String,
    source: String,
    control: ExpectedControl,
    mutations: Vec<ExpectedMutation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedControl {
    path: String,
    findings: usize,
    duplicates: usize,
    maximum_duration_ms: u128,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedMutation {
    id: String,
    path: String,
    viewport: u32,
    finding: String,
    primary_findings: usize,
    unexpected_primary_findings: usize,
    duplicates: usize,
    maximum_duration_ms: u128,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Qualification {
    cases: Vec<CaseResult>,
    comparisons: usize,
    passed_comparisons: usize,
    detected_mutations: usize,
    total_mutations: usize,
    maximum_ms: u128,
    p95_ms: u128,
    p99_ms: u128,
    duplicate_findings: usize,
    process_evidence: Option<ProcessEvidence>,
    failures: Vec<String>,
}

#[derive(Serialize)]
struct CaseResult {
    case: String,
    control_durations_ms: Vec<u128>,
    mutations: Vec<MutationResult>,
}

#[derive(Serialize)]
struct MutationResult {
    id: String,
    iterations: usize,
    durations_ms: Vec<u128>,
    finding: String,
}

pub async fn qualify(
    fixtures: &Path,
    output: Option<&Path>,
    repeat: usize,
    browser_path: Option<&Path>,
    recreate_bin: Option<&Path>,
    recreate_args: &[String],
) -> anyhow::Result<Qualification> {
    let repeat = repeat.max(1);
    let cases = load_cases(fixtures)?;
    anyhow::ensure!(!cases.is_empty(), "no fixture cases found");
    let server = Server::start(fixtures.to_path_buf()).await?;
    let executable = browser::find(browser_path)?;
    let profile_root = output
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("target"))
        .join("qualification-profiles");
    if profile_root.exists() {
        fs::remove_dir_all(&profile_root)?;
    }
    let source_process = browser::launch(&executable, &profile_root.join("source"), false).await?;
    let candidate_process =
        browser::launch(&executable, &profile_root.join("candidate"), false).await?;
    let (source_browser, candidate_browser) = tokio::try_join!(
        browser::version(&source_process.endpoint),
        browser::version(&candidate_process.endpoint)
    )?;
    anyhow::ensure!(
        source_browser == candidate_browser,
        "qualification browsers differ"
    );
    let (source_target, candidate_target) = tokio::try_join!(
        browser::create(&source_process.endpoint, "about:blank"),
        browser::create(&candidate_process.endpoint, "about:blank")
    )?;

    let process_evidence = recreate_bin
        .map(|path| blackbox::run(path, recreate_args))
        .transpose()?;
    let mut all_durations = Vec::new();
    let mut failures = Vec::new();
    let mut duplicate_findings = 0;
    let mut passed_comparisons = 0;
    let mut detected_mutations = 0;
    let total_mutations = cases.iter().map(|case| case.mutations.len()).sum();
    let mut case_results = Vec::new();
    for case in cases {
        anyhow::ensure!(
            case.schema_version == SCHEMA_VERSION,
            "unsupported expected fixture schema"
        );
        let source_url = format!(
            "{}/{}/{}",
            server.base_url,
            case.case,
            case.source.trim_start_matches('/')
        );
        let source_process_ref = &source_process;
        let source_target_id = source_target.id.as_str();
        let source_browser_ref = source_browser.as_str();
        // Recorded per iteration so the gate can detect nondeterminism in source
        // recording, not only in comparison.
        let record_source = |width: u32| {
            let source_url = source_url.clone();
            async move {
                let source_session = session(
                    Side::Source,
                    source_process_ref,
                    source_target_id,
                    &source_url,
                    source_browser_ref,
                    width,
                )?;
                capture::record_source(&source_session, false).await
            }
        };

        let control_url = format!(
            "{}/{}/{}",
            server.base_url,
            case.case,
            case.control.path.trim_start_matches('/')
        );
        let mut control_durations = Vec::new();
        let mut stable_control = None;
        for _ in 0..repeat {
            let artifact = record_source(1440).await?;
            let candidate_session = session(
                Side::Candidate,
                &candidate_process,
                &candidate_target.id,
                &control_url,
                &candidate_browser,
                1440,
            )?;
            let result = run_one(&artifact, &candidate_session, None).await;
            let report = result.report;
            all_durations.push(report.elapsed_ms);
            control_durations.push(report.elapsed_ms);
            let normalized = normalized_json(&report)?;
            let stable = stable_control.get_or_insert_with(|| normalized.clone());
            let valid = report.status == Status::Pass
                && report.findings.len() == case.control.findings
                && compare::duplicate_keys(&report.findings) == case.control.duplicates
                && report.elapsed_ms <= case.control.maximum_duration_ms
                && result.under_five_seconds
                && *stable == normalized;
            if valid {
                passed_comparisons += 1;
            } else {
                failures.push(format!(
                    "{} control: {}",
                    case.case,
                    report::text(&report).trim()
                ));
            }
        }

        let mut mutation_results = Vec::new();
        for mutation in &case.mutations {
            let candidate_url = format!(
                "{}/{}/{}",
                server.base_url,
                case.case,
                mutation.path.trim_start_matches('/')
            );
            let mut durations = Vec::new();
            let mut stable_text = None;
            let mut stable_json = None;
            let mut mutation_valid = true;
            for _ in 0..repeat {
                let artifact = record_source(mutation.viewport).await?;
                let candidate_session = session(
                    Side::Candidate,
                    &candidate_process,
                    &candidate_target.id,
                    &candidate_url,
                    &candidate_browser,
                    mutation.viewport,
                )?;
                let result = run_one(&artifact, &candidate_session, None).await;
                let report = result.report;
                let duplicates = compare::duplicate_keys(&report.findings);
                duplicate_findings += duplicates;
                let lines = report::text(&report);
                let finding = report
                    .findings
                    .first()
                    .map(|finding| finding.line.as_str())
                    .unwrap_or_default();
                let normalized = normalized_json(&report)?;
                let expected = report.status == Status::Fail
                    && report.findings.len() == mutation.primary_findings
                    && mutation.unexpected_primary_findings == 0
                    && duplicates == mutation.duplicates
                    && finding == mutation.finding
                    && report.elapsed_ms <= mutation.maximum_duration_ms
                    && result.under_five_seconds
                    && stable_text.as_ref().is_none_or(|value| value == &lines)
                    && stable_json
                        .as_ref()
                        .is_none_or(|value| value == &normalized);
                if expected {
                    passed_comparisons += 1;
                } else {
                    mutation_valid = false;
                    failures.push(format!(
                        "{} {}: expected={} actual={} status={:?} findings={} duplicates={} lines={:?}",
                        case.case,
                        mutation.id,
                        mutation.finding,
                        finding,
                        report.status,
                        report.findings.len(),
                        duplicates,
                        report
                            .findings
                            .iter()
                            .map(|value| value.line.as_str())
                            .collect::<Vec<_>>()
                    ));
                }
                stable_text.get_or_insert(lines);
                stable_json.get_or_insert(normalized);
                durations.push(report.elapsed_ms);
                all_durations.push(report.elapsed_ms);
            }
            if mutation_valid {
                detected_mutations += 1;
            }
            mutation_results.push(MutationResult {
                id: mutation.id.clone(),
                iterations: repeat,
                durations_ms: durations,
                finding: mutation.finding.clone(),
            });
        }
        case_results.push(CaseResult {
            case: case.case,
            control_durations_ms: control_durations,
            mutations: mutation_results,
        });
    }
    all_durations.sort_unstable();
    let p95 = percentile(&all_durations, 0.95);
    let p99 = percentile(&all_durations, 0.99);
    let maximum = all_durations.last().copied().unwrap_or_default();
    if p95 > 4000 {
        failures.push(format!("p95 {p95}ms > 4000ms"));
    }
    if p99 > 4500 {
        failures.push(format!("p99 {p99}ms > 4500ms"));
    }
    if maximum >= 5000 {
        failures.push(format!("maximum {maximum}ms >= 5000ms"));
    }
    if duplicate_findings != 0 {
        failures.push(format!("duplicate findings: {duplicate_findings}"));
    }
    if detected_mutations != total_mutations {
        failures.push(format!(
            "detected {detected_mutations}/{total_mutations} mutations"
        ));
    }
    if process_evidence
        .as_ref()
        .is_some_and(|value| !value.success)
    {
        failures.push("selected Recreate child process failed".into());
    }
    let qualification = Qualification {
        comparisons: all_durations.len(),
        passed_comparisons,
        detected_mutations,
        total_mutations,
        maximum_ms: maximum,
        p95_ms: p95,
        p99_ms: p99,
        duplicate_findings,
        process_evidence,
        cases: case_results,
        failures,
    };
    let bytes = serde_json::to_vec_pretty(&qualification)?;
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &bytes)?;
    }
    println!("{}", String::from_utf8_lossy(&bytes));
    anyhow::ensure!(
        qualification.failures.is_empty(),
        "qualification failed: {}",
        qualification.failures.join("; ")
    );
    Ok(qualification)
}

pub async fn compare_once(artifact: &Artifact, session: &Session) -> Report {
    run_one(artifact, session, None).await.report
}

pub async fn compare_once_focused(artifact: &Artifact, session: &Session, focus: &str) -> Report {
    run_one(artifact, session, Some(focus)).await.report
}

/// Compares an already-prepared candidate tab against snapshot source evidence
/// without navigating either page.
pub async fn compare_snapshot(
    artifact: &Artifact,
    session: &Session,
    focus: Option<&str>,
) -> (Report, Option<Artifact>) {
    let started = Instant::now();
    let deadline = Deadline::new(COMPARISON_DEADLINE_MS);
    let target = match deadline
        .run(
            "candidate preparation validation",
            capture::validate_candidate(artifact, session),
        )
        .await
    {
        Ok(target) => target,
        Err(error) => {
            return (
                compare::preparation_required_session(
                    artifact.digest.clone(),
                    started.elapsed().as_millis(),
                    error.to_string(),
                ),
                None,
            );
        }
    };
    match capture::compare_candidate_snapshot(artifact, session, target, deadline).await {
        Ok(actual) => {
            let report = match focus {
                Some(focus) => compare::artifacts_focused(
                    artifact,
                    &actual,
                    started.elapsed().as_millis(),
                    focus,
                ),
                None => compare::artifacts(artifact, &actual, started.elapsed().as_millis()),
            };
            (report, Some(actual))
        }
        Err(error) => (
            compare::inconclusive(
                artifact.digest.clone(),
                started.elapsed().as_millis(),
                error.to_string(),
            ),
            None,
        ),
    }
}

struct TimedReport {
    report: Report,
    under_five_seconds: bool,
}

async fn run_one(artifact: &Artifact, session: &Session, focus: Option<&str>) -> TimedReport {
    let started = Instant::now();
    // Leave time for the CLI to persist an INCONCLUSIVE report before its 4.8s watchdog.
    let deadline = Deadline::new(COMPARISON_DEADLINE_MS);
    let target = match deadline
        .run(
            "candidate preparation validation",
            capture::validate_candidate(artifact, session),
        )
        .await
    {
        Ok(target) => target,
        Err(error) => {
            return TimedReport {
                report: compare::preparation_required_session(
                    artifact.digest.clone(),
                    started.elapsed().as_millis(),
                    error.to_string(),
                ),
                under_five_seconds: started.elapsed().as_millis() < 5000,
            };
        }
    };
    let report = match capture::compare_candidate(artifact, session, target, deadline).await {
        Ok(actual) => match focus {
            Some(focus) => {
                compare::artifacts_focused(artifact, &actual, started.elapsed().as_millis(), focus)
            }
            None => compare::artifacts(artifact, &actual, started.elapsed().as_millis()),
        },
        Err(error) => compare::inconclusive(
            artifact.digest.clone(),
            started.elapsed().as_millis(),
            error.to_string(),
        ),
    };
    TimedReport {
        report,
        under_five_seconds: started.elapsed().as_millis() < 5000,
    }
}

fn session(
    side: Side,
    process: &browser::BrowserProcess,
    target_id: &str,
    url: &str,
    browser_name: &str,
    width: u32,
) -> anyhow::Result<Session> {
    let mut session = Session {
        schema_version: SCHEMA_VERSION,
        side,
        cdp_url: process.endpoint.clone(),
        target_id: target_id.into(),
        requested_url: url.into(),
        rendered_url: url.into(),
        browser: browser_name.into(),
        executable: process.executable.display().to_string(),
        profile: process.profile.display().to_string(),
        viewport: Viewport { width, height: 900 },
        digest: String::new(),
    };
    session.seal()?;
    Ok(session)
}

fn load_cases(root: &Path) -> anyhow::Result<Vec<ExpectedCase>> {
    let mut values: Vec<ExpectedCase> = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path().join("expected.json");
        if path.exists() {
            values.push(serde_json::from_slice(&fs::read(path)?)?);
        }
    }
    values.sort_by(|left, right| left.case.cmp(&right.case));
    Ok(values)
}

fn normalized_json(report: &Report) -> anyhow::Result<Vec<u8>> {
    let mut normalized = report.clone();
    normalized.elapsed_ms = 0;
    normalized.source_digest.clear();
    normalized.candidate_digest.clear();
    Ok(serde_json::to_vec(&normalized)?)
}

fn percentile(values: &[u128], percentile: f64) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() - 1) as f64 * percentile).ceil() as usize;
    values[index]
}
