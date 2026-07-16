use super::responsive_runtime_support::{assert_clean_events, assert_no_overflow, connect};
use super::structural_runtime_support::assert_structural_parity;
use crate::{
    browser, capture,
    cdp::Cdp,
    interactions,
    model::{PageState, Viewport},
};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::path::Path;

const CAPTURES: [(u32, u32); 2] = [(1200, 800), (390, 844)];

#[tokio::test]
#[ignore = "requires RECREATE_CDP_URL and RECREATE_STRUCTURE_URL"]
async fn generated_runtime_preserves_viewport_structures() -> Result<()> {
    let endpoint = required("RECREATE_CDP_URL")?;
    let runtime_url = required("RECREATE_STRUCTURE_URL")?;
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test/fixtures/responsive-structure.html");
    let source_url = url::Url::from_file_path(fixture).unwrap().to_string();
    let mut source = connect(&source_url, &endpoint).await?;
    let baselines = capture_states(&mut source).await?;
    let captured = interactions::capture(&mut source, &baselines).await?;
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].states.len(), 2);
    let mut runtime = connect(&runtime_url, &endpoint).await?;
    let generated = capture_states(&mut runtime).await?;
    for (expected, actual) in baselines.iter().zip(&generated) {
        assert_structural_parity(expected, actual);
    }
    validate_interaction(&mut runtime, &captured[0].states).await?;
    validate_boundaries(&mut runtime).await?;
    validate_reduced_motion(&mut runtime).await?;
    assert_clean_events(&mut runtime);
    Ok(())
}

async fn capture_states(cdp: &mut Cdp) -> Result<Vec<PageState>> {
    let mut states = Vec::new();
    for (index, (width, height)) in CAPTURES.into_iter().enumerate() {
        let viewport = viewport(width, height);
        states.push(capture::capture_state(cdp, viewport, index == 0).await?);
        assert_no_overflow(cdp, width).await?;
    }
    Ok(states)
}

async fn validate_interaction(cdp: &mut Cdp, expected: &[PageState]) -> Result<()> {
    load(cdp, 1200, 800).await?;
    press(cdp, "Tab", "Tab", 9).await?;
    press(cdp, "Enter", "Enter", 13).await?;
    ready(cdp).await?;
    assert_eq!(expanded(cdp).await?, "true");
    assert_eq!(role(cdp).await?, "dialog");
    let desktop = capture::read_state(cdp, viewport(1200, 800)).await?;
    assert_structural_parity(&expected[0], &desktop);
    load_width(cdp, 390, 844).await?;
    assert_eq!(expanded(cdp).await?, "true");
    assert_eq!(role(cdp).await?, "dialog");
    let mobile = capture::read_state(cdp, viewport(390, 844)).await?;
    assert_structural_parity(&expected[1], &mobile);
    press(cdp, "Escape", "Escape", 27).await?;
    ready(cdp).await?;
    assert_eq!(expanded(cdp).await?, "false");
    assert_eq!(role(cdp).await?, "button");
    Ok(())
}

async fn validate_boundaries(cdp: &mut Cdp) -> Result<()> {
    for (width, expected) in [(390, "Trips"), (391, "Upcoming trips")] {
        load(cdp, width, 800).await?;
        let text = cdp
            .evaluate("document.querySelector('h1').textContent")
            .await?;
        assert_eq!(text.as_str(), Some(expected));
        assert_no_overflow(cdp, width).await?;
    }
    Ok(())
}

async fn validate_reduced_motion(cdp: &mut Cdp) -> Result<()> {
    cdp.send(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    load(cdp, 390, 844).await?;
    press(cdp, "Tab", "Tab", 9).await?;
    press(cdp, "Enter", "Enter", 13).await?;
    ready(cdp).await?;
    assert_eq!(
        string(
            cdp,
            "String(matchMedia('(prefers-reduced-motion: reduce)').matches)"
        )
        .await?,
        "true"
    );
    assert_eq!(duration(cdp).await?, "0s");
    Ok(())
}

async fn load(cdp: &mut Cdp, width: u32, height: u32) -> Result<()> {
    load_width(cdp, width, height).await?;
    cdp.send("Page.reload", json!({"ignoreCache":false}))
        .await?;
    ready(cdp).await
}

async fn load_width(cdp: &mut Cdp, width: u32, height: u32) -> Result<()> {
    browser::set_viewport(cdp, width, height).await?;
    ready(cdp).await
}

async fn ready(cdp: &mut Cdp) -> Result<()> {
    cdp.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))")
        .await?;
    Ok(())
}

async fn press(cdp: &mut Cdp, key: &str, code: &str, value: u32) -> Result<()> {
    let common =
        json!({"key":key,"code":code,"windowsVirtualKeyCode":value,"nativeVirtualKeyCode":value});
    send_key(cdp, &common, json!({"type":"rawKeyDown"})).await?;
    if key == "Enter" || key == " " {
        let text = if key == "Enter" { "\r" } else { " " };
        send_key(cdp, &common, json!({"type":"char","text":text})).await?;
    }
    send_key(cdp, &common, json!({"type":"keyUp"})).await
}

async fn send_key(cdp: &mut Cdp, common: &Value, event: Value) -> Result<()> {
    let mut parameters = common.as_object().unwrap().clone();
    parameters.extend(event.as_object().unwrap().clone());
    cdp.send("Input.dispatchKeyEvent", Value::Object(parameters))
        .await?;
    Ok(())
}

async fn expanded(cdp: &mut Cdp) -> Result<String> {
    string(
        cdp,
        "document.querySelector('button').getAttribute('aria-expanded')",
    )
    .await
}

async fn role(cdp: &mut Cdp) -> Result<String> {
    string(
        cdp,
        "document.activeElement.getAttribute('role')||document.activeElement.tagName.toLowerCase()",
    )
    .await
}

async fn duration(cdp: &mut Cdp) -> Result<String> {
    string(
        cdp,
        "getComputedStyle(document.querySelector('[role=dialog]')).transitionDuration",
    )
    .await
}

async fn string(cdp: &mut Cdp, expression: &str) -> Result<String> {
    Ok(cdp
        .evaluate(expression)
        .await?
        .as_str()
        .context("expected string")?
        .into())
}

fn viewport(width: u32, height: u32) -> Viewport {
    Viewport {
        width,
        height,
        dpr: 1.0,
    }
}

fn required(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} is required"))
}
