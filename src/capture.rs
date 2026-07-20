use crate::{
    browser,
    capture_startup::{ensure_settled, startup_nodes, wait_ready, wait_startup},
    cli::CaptureArgs,
    generate, interactions, lifecycle_script,
    model::{BrowserCookie, PageState, Specification, Viewport},
    page_script, probe, validate,
};
use anyhow::{Context, Result};
use base64::Engine;
use serde_json::json;
use std::fs;
pub async fn run(args: CaptureArgs) -> Result<()> {
    let capture_started = std::time::Instant::now();
    let viewports = probe::parse_viewports(&args.viewports)?;
    fs::create_dir_all(&args.out)?;
    let (target, mut cdp) = browser::target(&args).await?;
    cdp.enable(&["Page", "Runtime", "Network", "DOM", "CSS"])
        .await?;
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({ "source": lifecycle_script::SOURCE }),
    )
    .await?;
    if args.reuse {
        cdp.evaluate(lifecycle_script::SOURCE).await?;
    }
    set_motion(&mut cdp).await?;
    let requested_url = args.url.clone().unwrap_or_else(|| target.url.clone());
    let mut states: Vec<PageState> = Vec::new();
    for viewport in viewports {
        let state_started = std::time::Instant::now();
        let reload = !args.reuse || args.reload;
        let state = if args.reuse && args.reload && states.is_empty() {
            capture_state_with_startup(&mut cdp, viewport.clone(), states.is_empty()).await?
        } else if !states.is_empty() {
            capture_state_without_assets(&mut cdp, viewport.clone(), false).await?
        } else {
            capture_state(&mut cdp, viewport.clone(), reload && states.is_empty()).await?
        };
        let screenshot = cdp
            .send("Page.captureScreenshot", json!({ "format": "png" }))
            .await?;
        if let Some(data) = screenshot["data"].as_str() {
            fs::write(
                args.out
                    .join(format!("source-{}x{}.png", viewport.width, viewport.height)),
                base64::engine::general_purpose::STANDARD.decode(data)?,
            )?;
        }
        let mut state = state;
        let paths = state
            .nodes
            .iter()
            .chain(&state.startup_nodes)
            .map(|node| node.path.as_str())
            .collect::<std::collections::HashSet<_>>();
        state
            .animations
            .retain(|animation| paths.contains(animation.target.as_str()));
        if let Some(primary) = states.first() {
            let targets: std::collections::HashSet<_> =
                state.nodes.iter().map(|node| node.path.as_str()).collect();
            state.attribute_sequences = primary
                .attribute_sequences
                .iter()
                .filter(|sequence| targets.contains(sequence.target.as_str()))
                .cloned()
                .collect();
            state
                .asset_data
                .retain(|url, data| primary.asset_data.get(url) != Some(data));
        }
        states.push(state);
        eprintln!(
            "captured {}x{} in {:.2}s",
            viewport.width,
            viewport.height,
            state_started.elapsed().as_secs_f64()
        );
    }
    let captured_url = states
        .first()
        .map(|state| state.url.clone())
        .unwrap_or_else(|| requested_url.clone());
    let interactions_started = std::time::Instant::now();
    let interaction_states = interactions::capture(&mut cdp, &states).await?;
    eprintln!(
        "captured interactions in {:.2}s",
        interactions_started.elapsed().as_secs_f64()
    );
    let mut specification = Specification {
        schema_version: 1,
        requested_url,
        captured_url,
        states,
        interactions: interaction_states,
    };
    crate::interaction_surface::normalize(&mut specification);
    fs::write(
        args.out.join("spec.json"),
        serde_json::to_vec_pretty(&specification)?,
    )?;
    let cookies = browser_cookies(&mut cdp).await;
    generate::write_project(&specification, &args.out, &cookies).await?;
    let acceptance = validate::validate(&specification, &args.out)?;
    fs::write(
        args.out.join("acceptance.json"),
        serde_json::to_vec_pretty(&acceptance)?,
    )?;
    if !acceptance.passed {
        anyhow::bail!("capture validation failed");
    }
    eprintln!(
        "completed capture in {:.2}s",
        capture_started.elapsed().as_secs_f64()
    );
    println!("{}", serde_json::to_string_pretty(&acceptance)?);
    Ok(())
}

pub async fn capture_state(
    cdp: &mut crate::cdp::Cdp,
    viewport: Viewport,
    reload: bool,
) -> Result<PageState> {
    prepare_state(cdp, &viewport, reload).await?;
    let state = read_state(cdp, viewport).await?;
    ensure_settled(&state)?;
    Ok(state)
}

async fn capture_state_without_assets(
    cdp: &mut crate::cdp::Cdp,
    viewport: Viewport,
    reload: bool,
) -> Result<PageState> {
    prepare_state(cdp, &viewport, reload).await?;
    let raw = cdp.evaluate(&page_script::source_without_assets()).await?;
    let text = raw
        .as_str()
        .context("responsive capture returned non-string")?;
    let mut state: PageState = serde_json::from_str(text)?;
    state.viewport = viewport;
    ensure_settled(&state)?;
    Ok(state)
}
pub async fn prepare_state(
    cdp: &mut crate::cdp::Cdp,
    viewport: &Viewport,
    reload: bool,
) -> Result<()> {
    browser::set_viewport(cdp, viewport.width, viewport.height).await?;
    if reload {
        cdp.send("Page.reload", json!({ "ignoreCache": false }))
            .await?;
    }
    clear_input_state(cdp).await?;
    wait_ready(cdp, true).await?;
    Ok(())
}

async fn capture_state_with_startup(
    cdp: &mut crate::cdp::Cdp,
    viewport: Viewport,
    observe_dynamic: bool,
) -> Result<PageState> {
    browser::set_viewport(cdp, viewport.width, viewport.height).await?;
    let started = std::time::Instant::now();
    cdp.send("Page.reload", json!({ "ignoreCache": false }))
        .await?;
    clear_input_state(cdp).await?;
    let startup = wait_startup(cdp, &viewport, started).await?;
    wait_ready(cdp, startup.is_some()).await?;
    let startup_elapsed = started.elapsed().as_millis() as u64;
    if observe_dynamic {
        tokio::time::sleep(std::time::Duration::from_millis(9_000)).await;
    }
    let mut state = read_state(cdp, viewport).await?;
    ensure_settled(&state)?;
    if let Some((startup_state, delay)) = startup {
        let settled: std::collections::BTreeSet<_> =
            state.nodes.iter().map(|node| node.path.as_str()).collect();
        let missing: Vec<_> = state
            .animations
            .iter()
            .filter(|animation| !settled.contains(animation.target.as_str()))
            .map(|animation| animation.target.clone())
            .collect();
        state.startup_nodes = startup_nodes(&startup_state, &missing);
        state.startup_delay_ms = delay;
        state.startup_duration_ms = startup_elapsed - delay;
        let startup: std::collections::BTreeSet<_> = state
            .startup_nodes
            .iter()
            .map(|node| node.path.as_str())
            .collect();
        for animation in &mut state.animations {
            if !settled.contains(animation.target.as_str()) {
                let target = format!("startup>{}", animation.target);
                if startup.contains(target.as_str()) {
                    animation.target = target;
                }
            }
        }
        state.animations.retain(|animation| {
            settled.contains(animation.target.as_str())
                || startup.contains(animation.target.as_str())
        });
        for url in startup_state.asset_urls {
            if !state.asset_urls.contains(&url) {
                state.asset_urls.push(url);
            }
        }
        state.asset_data.extend(startup_state.asset_data);
    }
    Ok(state)
}

async fn set_motion(cdp: &mut crate::cdp::Cdp) -> Result<()> {
    cdp.send(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"no-preference"}]}),
    )
    .await?;
    Ok(())
}

async fn clear_input_state(cdp: &mut crate::cdp::Cdp) -> Result<()> {
    let mut moved = false;
    for _ in 0..2 {
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            cdp.send(
                "Input.dispatchMouseEvent",
                json!({"type":"mouseMoved","x":-100,"y":-100}),
            ),
        )
        .await
        {
            Ok(result) => {
                result?;
                moved = true;
                break;
            }
            Err(_) => continue,
        }
    }
    if !moved {
        anyhow::bail!("CDP pointer reset timed out after two attempts");
    }
    cdp.evaluate("document.activeElement?.blur()").await?;
    Ok(())
}

pub async fn read_state(cdp: &mut crate::cdp::Cdp, viewport: Viewport) -> Result<PageState> {
    let source = page_script::source();
    let raw = cdp.evaluate(&source).await?;
    let text = raw.as_str().context("capture script returned non-string")?;
    let mut state: PageState = serde_json::from_str(text)?;
    state.viewport = viewport;
    Ok(state)
}

pub async fn read_interaction_state(
    cdp: &mut crate::cdp::Cdp,
    viewport: Viewport,
) -> Result<PageState> {
    let raw = cdp.evaluate(&crate::interaction_script::source()).await?;
    let text = raw
        .as_str()
        .context("interaction capture returned non-string")?;
    let mut state: PageState = serde_json::from_str(text)?;
    state.viewport = viewport;
    Ok(state)
}

async fn browser_cookies(cdp: &mut crate::cdp::Cdp) -> Vec<BrowserCookie> {
    cdp.send("Network.getAllCookies", json!({}))
        .await
        .ok()
        .and_then(|value| serde_json::from_value(value["cookies"].clone()).ok())
        .unwrap_or_default()
}
