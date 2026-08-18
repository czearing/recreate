//! The single owner of "what did this page show before it changed?".
//!
//! A page can show one thing while it loads and another once it is ready. That loading phase
//! is a fact about **time** — it exists because the page changed, not because whatever it
//! showed happened to be shaped a particular way. Recording it used to be entered only when
//! some element matched the full-viewport blocking-overlay predicate, so a splash curtain was
//! recorded and an inline skeleton — static, unstacked, card-sized, and the dominant
//! first-paint shape on real product pages — was not. The property that decided whether the
//! phase was recorded at all was how the placeholder had been styled.
//!
//! The trigger here is the phase itself. The page is read once at first paint, and whatever
//! is gone by the time it settles is what the phase consisted of. That one rule subsumes both
//! of the hand-written root sets it replaces: a curtain qualifies because it was removed, not
//! because it was positioned, and a vanished animation target is the same rule restricted to
//! elements that happened to animate. It is also strictly conservative — a page that only
//! *adds* nodes leaves the removed set empty and gains no startup layer — so
//! `blocking_overlay::is_blocking_overlay` keeps both of its own jobs and no page newly aborts.
//!
//! The read must happen inside the page. Driving it from Rust puts a round trip between
//! deciding to read and reading, making the result a guess about wall-clock time on a page
//! the tool has never seen.
//!
//! Ordering it is the whole difficulty. An injected script runs before any document script
//! and frame callbacks run in registration order, so a callback registered here precedes any
//! the page can register. But the earliest frames land while the document is still parsing,
//! when there is nothing to read; and waiting for `DOMContentLoaded` is too late, because a
//! page can paint and mutate across two frames before parsing ends — measured as one capture
//! in three recording no phase at all. So the reader re-arms frame by frame until the body
//! has something in it. Precedence survives inductively: each re-arm is registered from
//! inside a frame callback, and a page that had not registered its own callback by then
//! cannot register one earlier. Parsing finishing ends the wait, so a genuinely empty body
//! is read once rather than watched forever.

use crate::{
    cdp::Cdp,
    first_paint_assets,
    model::{Node, PageState, Viewport},
};
use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeSet;

/// Where the page leaves its first-paint reading for the driver to collect after settling.
const STASH: &str = "__recreateFirstPaint";

/// The grain the replay timings are recorded at.
///
/// Both numbers are wall-clock measurements taken across process scheduling, a navigation and
/// a frame boundary, so their low digits are jitter rather than signal — two captures of the
/// same page measured 577ms and 556ms for the same phase. Emitting that at millisecond
/// precision claims a resolution the measurement does not have, and bakes the noise into
/// generated source, so every recapture of an unchanged page reports a spurious change and
/// the one thing a corpus sweep exists to detect is buried. Rounding to a tenth of a second
/// keeps every difference the phase actually has: a phase whose length differs by less than
/// this is not a phase a viewer can tell apart.
const TIMING_GRAIN_MS: u64 = 100;

/// What the page recorded, and how long it then held it.
#[derive(Deserialize)]
pub struct FirstPaint {
    at_ms: u64,
    held_ms: u64,
    state: PageState,
}

/// The reader, armed so that it cannot lose a race with the page's own first frame.
///
/// Subresources are deliberately not downloaded here. The reading is a snapshot of a moment,
/// so it must not put requests in flight that the settle gate would then wait on, and the
/// content of anything the phase alone referenced is fetched afterwards instead, once it is
/// known which URLs the settled page did not already carry.
pub fn source() -> String {
    format!(
        "(()=>{{const painted=()=>document.body&&\
         (document.body.children.length>0||document.readyState!=='loading');\
         const arm=()=>requestAnimationFrame(()=>{{if(!painted())return arm();\
         window.{STASH}={{at:performance.now(),state:{capture}}};}});arm();}})();",
        capture = crate::page_script::source_at_first_paint(),
    )
}

/// Collects the reading the page left behind. Absent means the page never loaded under this
/// reader — a viewport change that reused an already-loaded page — not that it had no phase.
pub async fn collect(cdp: &mut Cdp, viewport: Viewport) -> Result<Option<FirstPaint>> {
    let raw = cdp
        .evaluate(&format!(
            "(async()=>{{const record=window.{STASH};if(!record)return null;\
             return JSON.stringify({{at_ms:Math.round(record.at),\
             held_ms:Math.round(performance.now()-record.at),\
             state:JSON.parse(await record.state)}});}})()"
        ))
        .await?;
    let Some(text) = raw.as_str() else {
        return Ok(None);
    };
    let mut record: FirstPaint = serde_json::from_str(text)?;
    record.state.viewport = viewport;
    Ok(Some(record))
}

/// The nodes the page showed at first paint and no longer shows once settled.
///
/// Every removed node is either a root of the phase or the descendant of one, so the set is
/// simply "removed" and roots need no separate pass: a root is recognisable afterwards as a
/// startup node whose parent did not come with it.
pub fn startup_nodes(first: &PageState, settled: &BTreeSet<&str>) -> Vec<Node> {
    let removed: BTreeSet<&str> = first
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .filter(|path| !settled.contains(path))
        .collect();
    first
        .nodes
        .iter()
        .filter(|node| removed.contains(node.path.as_str()))
        .cloned()
        .map(|mut node| {
            node.parent = node
                .parent
                .filter(|parent| removed.contains(parent.as_str()))
                .map(|parent| format!("startup>{parent}"));
            node.path = format!("startup>{}", node.path);
            node
        })
        .collect()
}

/// Folds a recorded phase into the settled state, or leaves the state untouched when the page
/// showed nothing it later took away.
pub async fn merge(cdp: &mut Cdp, state: &mut PageState, record: FirstPaint) -> Result<()> {
    let startup = startup_nodes(&record.state, &settled_paths(state));
    if startup.is_empty() {
        return Ok(());
    }
    state.startup_nodes = startup;
    state.startup_delay_ms = grain(record.at_ms);
    state.startup_duration_ms = grain(record.held_ms);
    retarget_animations(state);
    first_paint_assets::merge(cdp, state, record.state).await
}

/// A measured span at the precision it was actually measured to.
pub fn grain(ms: u64) -> u64 {
    (ms + TIMING_GRAIN_MS / 2) / TIMING_GRAIN_MS * TIMING_GRAIN_MS
}

fn settled_paths(state: &PageState) -> BTreeSet<&str> {
    state.nodes.iter().map(|node| node.path.as_str()).collect()
}

/// An animation whose target left with the phase still played during it, so it follows its
/// target into the startup layer; one whose target exists in neither layer has nothing left
/// to animate and is dropped.
fn retarget_animations(state: &mut PageState) {
    let settled: BTreeSet<String> = settled_paths(state).iter().map(|p| p.to_string()).collect();
    let startup: BTreeSet<String> = state
        .startup_nodes
        .iter()
        .map(|node| node.path.clone())
        .collect();
    for animation in &mut state.animations {
        if settled.contains(&animation.target) {
            continue;
        }
        let target = format!("startup>{}", animation.target);
        if startup.contains(&target) {
            animation.target = target;
        }
    }
    state.animations.retain(|animation| {
        settled.contains(&animation.target) || startup.contains(&animation.target)
    });
}

#[cfg(test)]
#[path = "first_paint_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "first_paint_reader_tests.rs"]
mod reader_tests;
