mod runtime;
mod support;
mod verify;

use crate::{browser, capture, cli::CaptureArgs, interactions, lifecycle_script};
use serde::Serialize;
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use support::{
    browser_path, collect_errors, directory_size, free_port, launch_browser, selected_fixtures,
    selected_viewports, specification, viewport, wait_for_browser,
};

const FIXTURES: &[&str] = &[
    "dashboard",
    "article",
    "form",
    "responsive-bands",
    "interaction",
];
const VIEWPORTS: &[(u32, u32)] = &[
    (1920, 1080),
    (1440, 900),
    (768, 1024),
    (390, 844),
    (320, 568),
];

#[derive(Serialize)]
struct Evidence {
    browser: String,
    viewports: &'static [(u32, u32)],
    fixtures: Vec<FixtureEvidence>,
    console_errors: usize,
    network_errors: usize,
}

#[derive(Serialize)]
struct FixtureEvidence {
    name: String,
    nodes: usize,
    parity_mismatches: usize,
    parity_details: Vec<String>,
    horizontal_overflows: usize,
    keyboard_activation: bool,
    focus_restoration: bool,
    reduced_motion: bool,
    capture_ms: u128,
    build_ms: u128,
    browser_ms: u128,
    source_bytes: u64,
    build_bytes: u64,
}

#[tokio::test]
async fn generic_browser_release_gate() {
    let browser_path = browser_path().expect("Chromium browser is required for release validation");
    let port = free_port();
    let profile = tempfile::tempdir().unwrap();
    let mut child = launch_browser(&browser_path, profile.path(), port);
    wait_for_browser(port).await;
    let result = run_gate(&browser_path, port).await;
    let _ = child.kill();
    let _ = child.wait();
    result.unwrap();
}

async fn run_gate(browser_path: &Path, port: u16) -> anyhow::Result<()> {
    let evidence_output = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release-gate");
    fs::create_dir_all(&evidence_output)?;
    let evidence_path = evidence_output.join("evidence.json");
    if evidence_path.exists() {
        fs::remove_file(&evidence_path)?;
    }
    let workspace = tempfile::tempdir()?;
    let mut evidence = Evidence {
        browser: browser_path.display().to_string(),
        viewports: VIEWPORTS,
        fixtures: Vec::new(),
        console_errors: 0,
        network_errors: 0,
    };
    for fixture in selected_fixtures(FIXTURES) {
        let result = validate_fixture(fixture, port, workspace.path()).await?;
        evidence.console_errors += result.1;
        evidence.network_errors += result.2;
        evidence.fixtures.push(result.0);
        write_evidence(&evidence_output, &evidence)?;
        let latest = evidence.fixtures.last().expect("fixture evidence");
        assert_eq!(
            latest.parity_mismatches, 0,
            "{fixture} generated parity: {:?}",
            latest.parity_details
        );
        assert_eq!(
            latest.horizontal_overflows, 0,
            "{fixture} horizontal overflow"
        );
        assert!(latest.keyboard_activation, "{fixture} keyboard activation");
        assert!(latest.focus_restoration, "{fixture} focus restoration");
        assert!(latest.reduced_motion, "{fixture} reduced motion");
    }
    assert_eq!(evidence.console_errors, 0, "browser console errors");
    assert_eq!(evidence.network_errors, 0, "browser network errors");
    Ok(())
}

fn write_evidence(output: &Path, evidence: &Evidence) -> anyhow::Result<()> {
    fs::write(
        output.join("evidence.json"),
        serde_json::to_vec_pretty(evidence)?,
    )?;
    Ok(())
}

async fn validate_fixture(
    name: &str,
    port: u16,
    workspace: &Path,
) -> anyhow::Result<(FixtureEvidence, usize, usize)> {
    let capture_started = Instant::now();
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test/fixtures")
        .join(format!("{name}.html"));
    let args = CaptureArgs {
        url: Some(url::Url::from_file_path(fixture).unwrap().to_string()),
        reuse: false,
        reload: false,
        baseline_only: false,
        spec_only: false,
        target: None,
        cdp_url: format!("http://127.0.0.1:{port}"),
        out: PathBuf::new(),
        viewports: String::new(),
    };
    let (_, mut cdp) = browser::target(&args).await?;
    cdp.enable(&["Page", "Runtime", "Network", "DOM", "CSS"])
        .await?;
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({"source": lifecycle_script::SOURCE}),
    )
    .await?;
    let mut states = Vec::new();
    for (index, (width, height)) in selected_viewports(VIEWPORTS).into_iter().enumerate() {
        states.push(capture::capture_state(&mut cdp, viewport(width, height), index == 0).await?);
    }
    let captured_interactions = if name == "interaction" {
        interactions::capture(&mut cdp, &states).await?
    } else {
        Vec::new()
    };
    let capture_ms = capture_started.elapsed().as_millis();
    let source_errors = collect_errors(&mut cdp);
    let specification = specification(states, captured_interactions);
    let directory = workspace.join(name);
    crate::generate::write_project(&specification, &directory, &[]).await?;
    assert!(crate::validate::validate(&specification, &directory)?.passed);
    let source_bytes = directory_size(&directory)?;
    let build_started = Instant::now();
    runtime::build(&directory.join("react"))?;
    let build_ms = build_started.elapsed().as_millis();
    let build_bytes = directory_size(&directory.join("react/dist"))?;
    let browser_started = Instant::now();
    let verified = verify::generated(&specification, &directory.join("react/dist"), port).await?;
    Ok((
        FixtureEvidence {
            name: name.into(),
            nodes: specification
                .states
                .iter()
                .map(|state| state.nodes.len())
                .sum(),
            parity_mismatches: verified.parity_mismatches,
            parity_details: verified.parity_details,
            horizontal_overflows: verified.horizontal_overflows,
            keyboard_activation: verified.keyboard_activation,
            focus_restoration: verified.focus_restoration,
            reduced_motion: verified.reduced_motion,
            capture_ms,
            build_ms,
            browser_ms: browser_started.elapsed().as_millis(),
            source_bytes,
            build_bytes,
        },
        source_errors.0 + verified.console_errors,
        source_errors.1 + verified.network_errors,
    ))
}
