use crate::{
    browser,
    cli::CaptureArgs,
    first_paint, generate, interactions, lifecycle_script,
    model::{PageState, Specification},
    probe, serve, validate,
};
use anyhow::Result;
use serde_json::json;
use std::fs;

#[path = "capture/state.rs"]
mod state;

#[path = "capture/dynamic.rs"]
mod dynamic;

#[path = "capture/authored_sheets.rs"]
pub(crate) mod authored_sheets;

use state::{browser_cookies, capture_state_without_assets, set_motion};

pub(crate) use state::set_focus;
pub use state::{capture_state, prepare_interaction_state, read_interaction_state};

/// Reading a page without the surrounding capture is something only the verification and
/// responsive-runtime harnesses need; the pipeline itself always goes through `capture_state`.
#[cfg(test)]
pub use state::read_state;

pub async fn run(mut args: CaptureArgs) -> Result<()> {
    let capture_started = std::time::Instant::now();
    let served = match args.url.as_deref().and_then(serve::as_directory) {
        Some(root) => {
            let served = serve::Directory::serve(&root).await?;
            args.url = Some(served.url.clone());
            Some(served)
        }
        None => None,
    };
    let mut session = browser::target(&args).await?;
    let outcome = capture_into(&mut session, &args, capture_started).await;
    session.close().await?;
    drop(served);
    outcome
}

/// The tab is opened by the caller and closed by the caller, so everything that
/// can fail lives here and reports its failure as a value rather than by
/// unwinding past the close.
async fn capture_into(
    session: &mut browser::Session,
    args: &CaptureArgs,
    capture_started: std::time::Instant,
) -> Result<()> {
    let viewports = probe::parse_viewports(&args.viewports)?;
    fs::create_dir_all(&args.out)?;
    let requested_url = args
        .url
        .clone()
        .unwrap_or_else(|| session.target.url.clone());
    let cdp = &mut session.cdp;
    cdp.enable(&["Page", "Runtime", "Network", "DOM", "CSS"])
        .await?;
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({ "source": lifecycle_script::source() }),
    )
    .await?;
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({ "source": first_paint::source() }),
    )
    .await?;
    if args.reuse {
        cdp.evaluate(&lifecycle_script::source()).await?;
    }
    set_focus(cdp).await?;
    set_motion(cdp).await?;
    let mut states: Vec<PageState> = Vec::new();
    for viewport in viewports {
        let state_started = std::time::Instant::now();
        let reload = !args.reuse || args.reload;
        let state = if states.is_empty() {
            capture_state(cdp, viewport.clone(), reload).await?
        } else {
            capture_state_without_assets(cdp, viewport.clone(), false).await?
        };
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
    let interaction_graph = if args.interactions {
        let interactions_started = std::time::Instant::now();
        let captured = interactions::capture_graph(cdp, &states).await?;
        eprintln!(
            "captured {} interactions in {:.2}s",
            captured.interactions.len(),
            interactions_started.elapsed().as_secs_f64()
        );
        captured
    } else {
        interactions::CapturedGraph {
            interactions: Vec::new(),
            transitions: Vec::new(),
        }
    };
    let mut specification = Specification {
        schema_version: 1,
        requested_url,
        captured_url,
        states,
        interactions: interaction_graph.interactions,
        transitions: interaction_graph.transitions,
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
    let cookies = browser_cookies(cdp).await;
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
