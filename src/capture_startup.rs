use crate::{
    blocking_overlay::{is_blocking_overlay, js_predicate},
    capture::read_state,
    capture_settle,
    cdp::Cdp,
    model::{PageState, Viewport},
};
use anyhow::Result;
use std::time::Duration;

/// How often the page is asked whether a startup curtain has appeared yet.
const STARTUP_POLL_MS: u64 = 100;
/// How long a page is given to raise a startup curtain before capture concludes it has none.
const STARTUP_ATTEMPTS: u32 = 60;
/// How long a curtain's own images are given to decode before it is judged still present.
const CURTAIN_IMAGE_MS: u64 = 2_000;

pub async fn wait_ready(cdp: &mut Cdp, wait_for_startup: bool) -> Result<()> {
    wait_ready_mode(cdp, wait_for_startup, true).await
}

pub async fn wait_ready_without_lifecycle(cdp: &mut Cdp, wait_for_startup: bool) -> Result<()> {
    wait_ready_mode(cdp, wait_for_startup, false).await
}

/// Asks the page once when it has settled, rather than interrogating it on a timer.
///
/// The page observes its own changes, so it can answer the moment it is still. Driving the
/// same question from Rust meant every answer was rounded up to the next poll interval, and
/// the cost of asking — a full style scan of every element — was paid on every tick instead
/// of only at the moments the answer could have changed.
async fn wait_ready_mode(
    cdp: &mut Cdp,
    wait_for_startup: bool,
    wait_for_lifecycle: bool,
) -> Result<()> {
    let settled = cdp
        .evaluate(&capture_settle::source(
            wait_for_lifecycle,
            wait_for_startup,
        ))
        .await?;
    anyhow::ensure!(
        settled.as_bool() == Some(true),
        "page did not become stable"
    );
    Ok(())
}

/// A settled capture that still holds a curtain recorded the splash screen, not the page.
pub fn ensure_settled(state: &PageState) -> Result<()> {
    if state
        .nodes
        .iter()
        .any(|node| is_blocking_overlay(node, &state.viewport))
    {
        anyhow::bail!("settled capture still contains a blocking overlay");
    }
    Ok(())
}

/// Waits for a startup curtain to appear and finish drawing, so the layer a page shows
/// first can be recorded before it is replaced.
pub async fn wait_startup(
    cdp: &mut Cdp,
    viewport: &Viewport,
    started: std::time::Instant,
) -> Result<Option<(PageState, u64)>> {
    let source = format!(
        "(async () => {{\
         const blocking = {predicate};\
         const overlay = Array.from(document.querySelectorAll('*')).find(blocking);\
         if (!overlay) return false;\
         const images = [...(overlay.matches('img') ? [overlay] : []), \
         ...overlay.querySelectorAll('img')];\
         await Promise.race([\
         Promise.all(images.map(image => image.complete\
         ? (image.decode ? image.decode().catch(() => {{}}) : Promise.resolve())\
         : new Promise(resolve => {{\
         image.addEventListener('load', resolve, {{ once: true }});\
         image.addEventListener('error', resolve, {{ once: true }});\
         }}))),\
         new Promise(resolve => setTimeout(resolve, {CURTAIN_IMAGE_MS}))]);\
         return blocking(overlay);\
         }})()",
        predicate = js_predicate(),
    );
    for _ in 0..STARTUP_ATTEMPTS {
        if cdp.evaluate(&source).await?.as_bool() == Some(true) {
            let state = read_state(cdp, viewport.clone()).await?;
            return Ok(Some((state, started.elapsed().as_millis() as u64)));
        }
        tokio::time::sleep(Duration::from_millis(STARTUP_POLL_MS)).await;
    }
    Ok(None)
}

/// Selects the subtree a startup curtain owns, so it can be recorded as its own layer.
pub fn startup_nodes(state: &PageState, animation_targets: &[String]) -> Vec<crate::model::Node> {
    let mut roots: Vec<_> = state
        .nodes
        .iter()
        .filter(|node| is_blocking_overlay(node, &state.viewport))
        .map(|node| node.path.clone())
        .collect();
    for target in animation_targets {
        if state.nodes.iter().any(|node| node.path == *target) && !roots.contains(target) {
            roots.push(target.clone());
        }
    }
    let selected: std::collections::BTreeSet<_> = state
        .nodes
        .iter()
        .filter(|node| {
            roots
                .iter()
                .any(|root| node.path == *root || node.path.starts_with(&format!("{root}>")))
        })
        .map(|node| node.path.clone())
        .collect();
    state
        .nodes
        .iter()
        .filter(|node| selected.contains(&node.path))
        .cloned()
        .map(|mut node| {
            let original = node.path.clone();
            node.path = format!("startup>{original}");
            node.parent = node
                .parent
                .filter(|parent| selected.contains(parent))
                .map(|parent| format!("startup>{parent}"));
            node
        })
        .collect()
}
