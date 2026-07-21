use crate::{browser, capture, cli::CaptureArgs, interactions, lifecycle_script, model::PageState};
use anyhow::{Context, Result};
use serde_json::json;
use std::{path::Path, time::Instant};

use super::{interaction_geometry_support as geometry, interaction_runtime_support as support};

const VIEWPORTS: [(u32, u32); 2] = [(1200, 800), (390, 844)];
const SETTLE: &str = r#"new Promise(resolve => {
  const start = performance.now();
  let cleanFrames = 0;
  const sample = () => {
    const running = document.getAnimations({subtree:true})
      .some(animation => animation.playState === 'running');
    cleanFrames = running ? 0 : cleanFrames + 1;
    const elapsed = performance.now() - start;
    if ((elapsed >= 50 && cleanFrames >= 2) || elapsed >= 500) resolve(elapsed);
    else requestAnimationFrame(sample);
  };
  requestAnimationFrame(sample);
})"#;

#[tokio::test]
#[ignore = "requires RECREATE_CDP_URL and RECREATE_DISCLOSURE_URL"]
async fn generated_disclosure_preserves_flow_and_accessibility() -> Result<()> {
    let endpoint = std::env::var("RECREATE_CDP_URL").context("RECREATE_CDP_URL is required")?;
    let runtime =
        std::env::var("RECREATE_DISCLOSURE_URL").context("RECREATE_DISCLOSURE_URL is required")?;
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test/fixtures/interaction-disclosure.html");
    let source_url = url::Url::from_file_path(fixture).unwrap().to_string();
    let mut source = connect(&source_url, &endpoint).await?;
    let mut baselines = Vec::new();
    for (width, height) in VIEWPORTS {
        baselines.push(
            capture::capture_state(&mut source, geometry::viewport(width, height), true).await?,
        );
    }
    let captured = interactions::capture(&mut source, &baselines).await?;
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].states.len(), VIEWPORTS.len());
    let source_errors = support::errors(&mut source);
    let mut generated = connect(&runtime, &endpoint).await?;
    geometry::validate_boundaries(&mut generated).await?;
    let started = Instant::now();
    for expected in &captured[0].states {
        validate_state(&mut generated, expected).await?;
    }
    assert!(started.elapsed().as_secs() < 20);
    validate_reduced_motion(&mut generated).await?;
    let generated_errors = support::errors(&mut generated);
    assert_eq!(source_errors, (0, 0));
    assert_eq!(generated_errors, (0, 0));
    Ok(())
}

#[tokio::test]
#[ignore = "requires RECREATE_CDP_URL and RECREATE_INTERACTION_URL"]
async fn generated_dialog_escape_restores_trigger_focus() -> Result<()> {
    let endpoint = std::env::var("RECREATE_CDP_URL").context("RECREATE_CDP_URL is required")?;
    let runtime = std::env::var("RECREATE_INTERACTION_URL")?;
    let mut generated = connect(&runtime, &endpoint).await?;
    geometry::load_state(&mut generated, &geometry::viewport(1200, 800)).await?;
    support::press(&mut generated, "Tab", "Tab", 9).await?;
    support::press(&mut generated, "Enter", "Enter", 13).await?;
    assert_eq!(active_role(&mut generated).await?, "button");
    assert!(support::active_in_dialog(&mut generated).await?);
    assert!(dialog_open(&mut generated).await?);
    support::press(&mut generated, "Escape", "Escape", 27).await?;
    generated
        .evaluate("new Promise(resolve => requestAnimationFrame(resolve))")
        .await?;
    assert!(!dialog_open(&mut generated).await?);
    assert_eq!(active_role(&mut generated).await?, "button");
    assert_eq!(
        support::active_attribute(&mut generated, "aria-expanded").await?,
        "false"
    );
    support::validate_modal_environment(&mut generated).await?;
    Ok(())
}

async fn validate_state(cdp: &mut crate::cdp::Cdp, expected: &PageState) -> Result<()> {
    geometry::load_state(cdp, &expected.viewport).await?;
    geometry::assert_body_width(cdp, expected.viewport.width).await?;
    support::press(cdp, "Tab", "Tab", 9).await?;
    assert_eq!(attribute(cdp, "aria-expanded").await?, "false");
    assert_eq!(active_role(cdp).await?, "button");
    support::press(cdp, " ", "Space", 32).await?;
    let settled = cdp
        .evaluate(SETTLE)
        .await?
        .as_f64()
        .context("settle duration")?;
    assert!(settled <= 500.0);
    assert_eq!(attribute(cdp, "aria-expanded").await?, "true");
    geometry::assert_body_width(cdp, expected.viewport.width).await?;
    assert_eq!(active_role(cdp).await?, "button");
    let actual = capture::read_state(cdp, expected.viewport.clone()).await?;
    support::assert_parity(expected, &actual);
    support::press(cdp, "Enter", "Enter", 13).await?;
    cdp.evaluate("new Promise(resolve => requestAnimationFrame(resolve))")
        .await?;
    assert_eq!(attribute(cdp, "aria-expanded").await?, "false");
    assert_eq!(active_role(cdp).await?, "button");
    Ok(())
}

async fn validate_reduced_motion(cdp: &mut crate::cdp::Cdp) -> Result<()> {
    cdp.send(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    geometry::load_state(cdp, &geometry::viewport(390, 844)).await?;
    support::press(cdp, "Tab", "Tab", 9).await?;
    support::press(cdp, "Enter", "Enter", 13).await?;
    let value = cdp
        .evaluate(
            "({matches:matchMedia('(prefers-reduced-motion: reduce)').matches,\
              duration:getComputedStyle(document.querySelector('#shipping-details')).transitionDuration})",
        )
        .await?;
    assert_eq!(value["matches"].as_bool(), Some(true));
    assert_eq!(value["duration"].as_str(), Some("0s"));
    Ok(())
}

async fn connect(url: &str, endpoint: &str) -> Result<crate::cdp::Cdp> {
    let args = CaptureArgs {
        url: Some(url.into()),
        reuse: false,
        reload: false,
        baseline_only: false,
        spec_only: false,
        target: None,
        cdp_url: endpoint.into(),
        out: Default::default(),
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
    Ok(cdp)
}

async fn attribute(cdp: &mut crate::cdp::Cdp, name: &str) -> Result<String> {
    Ok(cdp
        .evaluate(&format!(
            "document.querySelector('button').getAttribute({})",
            serde_json::to_string(name)?
        ))
        .await?
        .as_str()
        .context("missing attribute")?
        .into())
}

async fn active_role(cdp: &mut crate::cdp::Cdp) -> Result<String> {
    Ok(cdp
        .evaluate("document.activeElement.getAttribute('role') || document.activeElement.tagName.toLowerCase()")
        .await?
        .as_str()
        .context("missing active role")?
        .into())
}

async fn dialog_open(cdp: &mut crate::cdp::Cdp) -> Result<bool> {
    Ok(cdp
        .evaluate("document.querySelector('[role=dialog]') != null")
        .await?
        .as_bool()
        == Some(true))
}
