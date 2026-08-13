use crate::{
    browser,
    capture_settle::{note_curtain, wait_ready, wait_ready_without_lifecycle},
    first_paint,
    model::{PageState, Viewport},
    page_script,
};
use anyhow::{Context, Result};
use serde_json::json;

#[path = "read.rs"]
mod read;

pub(in crate::capture) use read::browser_cookies;
pub use read::{read_interaction_state, read_state};

/// The one full read of a page, including whatever it showed before it settled.
///
/// First-paint recording lives here rather than in a second capture routine of its own. The
/// duplicate existed once, reachable only from a flag combination the default capture never
/// set, so a page's loading phase went unrecorded even when a curtain matched — the predicate
/// everyone blamed was never consulted.
pub async fn capture_state(
    cdp: &mut crate::cdp::Cdp,
    viewport: Viewport,
    reload: bool,
) -> Result<PageState> {
    prepare_state(cdp, &viewport, reload).await?;
    let first = first_paint::collect(cdp, viewport.clone()).await?;
    let mut state = read_state(cdp, viewport).await?;
    note_curtain(&mut state);
    if let Some(record) = first {
        first_paint::merge(cdp, &mut state, record).await?;
    }
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
