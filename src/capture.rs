use crate::{
    browser,
    cli::CaptureArgs,
    generate, lifecycle_script,
    model::{PageState, Specification},
    probe, validate,
};
use anyhow::Result;
use base64::Engine;
use serde_json::json;
use std::fs;

#[path = "capture/state.rs"]
mod state;

#[path = "capture/dynamic.rs"]
mod dynamic;

#[path = "capture/authored_sheets.rs"]
mod authored_sheets;

use state::{
    browser_cookies, capture_state_with_startup, capture_state_without_assets, set_motion,
};

pub(crate) use state::set_focus;
pub use state::{capture_state, prepare_interaction_state, read_interaction_state, read_state};

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
    set_focus(&mut cdp).await?;
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
    let mut specification = Specification {
        schema_version: 1,
        requested_url,
        captured_url,
        states,
        interactions: Vec::new(),
        transitions: Vec::new(),
    };
    crate::interaction_surface::normalize(&mut specification);
    fs::write(
        args.out.join("spec.json"),
        serde_json::to_vec(&specification)?,
    )?;
    if args.spec_only {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "states": specification.states.len(),
                "interactions": specification.interactions.len(),
                "spec": args.out.join("spec.json"),
            }))?
        );
        return Ok(());
    }
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
