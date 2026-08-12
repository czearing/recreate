use crate::{
    browser,
    capture_startup::{
        note_curtain, startup_nodes, wait_ready, wait_ready_without_lifecycle, wait_startup,
    },
    model::{PageState, Viewport},
    page_script,
};
use anyhow::{Context, Result};
use serde_json::json;

#[path = "read.rs"]
mod read;

pub(in crate::capture) use read::browser_cookies;
pub use read::{read_interaction_state, read_state};

pub async fn capture_state(
    cdp: &mut crate::cdp::Cdp,
    viewport: Viewport,
    reload: bool,
) -> Result<PageState> {
    prepare_state(cdp, &viewport, reload).await?;
    let mut state = read_state(cdp, viewport).await?;
    note_curtain(&mut state);
    Ok(state)
}

pub(super) async fn capture_state_without_assets(
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
    note_curtain(&mut state);
    Ok(state)
}

pub async fn prepare_state(
    cdp: &mut crate::cdp::Cdp,
    viewport: &Viewport,
    reload: bool,
) -> Result<()> {
    prepare(cdp, viewport, reload).await?;
    wait_ready(cdp, true).await?;
    observe_dynamic(cdp, reload).await
}

/// A page's timed progression happens once, from a fresh load, so the recorder watches for
/// it exactly when the page was just loaded. A viewport change that reuses the page it has
/// already watched has nothing further to observe, and a page with no timed behaviour is
/// released within a frame, so the guard costs a static page nothing.
async fn observe_dynamic(cdp: &mut crate::cdp::Cdp, reload: bool) -> Result<()> {
    if reload {
        super::dynamic::observe(cdp).await?;
    }
    Ok(())
}

pub async fn prepare_interaction_state(
    cdp: &mut crate::cdp::Cdp,
    viewport: &Viewport,
    reload: bool,
) -> Result<()> {
    prepare(cdp, viewport, reload).await?;
    wait_ready_without_lifecycle(cdp, true).await
}

async fn prepare(cdp: &mut crate::cdp::Cdp, viewport: &Viewport, reload: bool) -> Result<()> {
    browser::set_viewport(cdp, viewport.width, viewport.height).await?;
    if reload {
        cdp.send("Page.reload", json!({ "ignoreCache": false }))
            .await?;
    }
    clear_input_state(cdp).await
}

pub(super) async fn capture_state_with_startup(
    cdp: &mut crate::cdp::Cdp,
    viewport: Viewport,
) -> Result<PageState> {
    browser::set_viewport(cdp, viewport.width, viewport.height).await?;
    let started = std::time::Instant::now();
    cdp.send("Page.reload", json!({ "ignoreCache": false }))
        .await?;
    clear_input_state(cdp).await?;
    let startup = wait_startup(cdp, &viewport, started).await?;
    wait_ready(cdp, startup.is_some()).await?;
    let startup_elapsed = started.elapsed().as_millis() as u64;
    observe_dynamic(cdp, true).await?;
    let mut state = read_state(cdp, viewport).await?;
    note_curtain(&mut state);
    if let Some((startup_state, delay)) = startup {
        merge_startup(&mut state, startup_state, delay, startup_elapsed);
    }
    Ok(state)
}

fn merge_startup(
    state: &mut PageState,
    startup_state: PageState,
    delay: u64,
    startup_elapsed: u64,
) {
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
        settled.contains(animation.target.as_str()) || startup.contains(animation.target.as_str())
    });
    for url in startup_state.asset_urls {
        if !state.asset_urls.contains(&url) {
            state.asset_urls.push(url);
        }
    }
    state.asset_data.extend(startup_state.asset_data);
}

pub(super) async fn set_motion(cdp: &mut crate::cdp::Cdp) -> Result<()> {
    cdp.send(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"no-preference"}]}),
    )
    .await?;
    Ok(())
}

/// A page in a window that is not in front is treated as hidden, so pointer
/// dispatch blocks for five seconds and animation frames are throttled.
/// Emulating focus keeps an unattended capture both fast and faithful.
pub(crate) async fn set_focus(cdp: &mut crate::cdp::Cdp) -> Result<()> {
    cdp.send(
        "Emulation.setFocusEmulationEnabled",
        json!({ "enabled": true }),
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
