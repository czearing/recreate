//! The single owner of "has this page stopped changing?".
//!
//! Settling used to be answered three times over: a Rust loop re-evaluating a full style
//! scan every 250ms, a repeat counter deciding when geometry had held still, and a separate
//! poll waiting for a startup curtain to leave. Every one of them measured the page through
//! a fixed interval, so the cheapest possible capture still paid two 250ms sleeps per
//! viewport before it was allowed to look, and a page that settled in 10ms waited 500.
//!
//! The page already knows when it changed. A `MutationObserver` reports DOM edits as they
//! happen, so quiet costs nothing to observe and is detected within one animation frame
//! instead of one poll interval. Layout that moves without a DOM edit — a CSS transition —
//! is caught by comparing the geometry signature across a single frame, and that scan runs
//! only at moments the DOM has already gone quiet rather than on every tick.

use crate::blocking_overlay;
use anyhow::Result;

/// How many consecutive animation frames must pass with no DOM edit before geometry is
/// worth reading. Two frames distinguish "nothing is happening" from "between two edits".
const QUIET_FRAMES: u32 = 2;
/// The longest a page may keep moving before it is captured mid-motion, for pages whose
/// geometry never repeats because something on them loops forever.
const STABLE_CEILING_MS: u64 = 8_000;
/// The longest a page may take to report itself loaded, or a startup curtain to leave,
/// before capture gives up entirely.
pub(crate) const READY_CEILING_MS: u64 = 30_000;
/// How long the transport waits for an injected script to answer.
///
/// This is derived rather than declared, because the two numbers describe one thing from
/// two ends. The settle probe is the longest-running script the tool injects, and the
/// ceiling above is a deliberate grant: a page that never goes quiet is still meant to be
/// captured once it expires. A transport deadline equal to that grant makes the grant
/// unreachable — the probe resolves at the same instant the client stops listening, so the
/// outcome the ceiling exists to produce is discarded by the caller that asked for it, and
/// every page that needs the full budget fails instead of being captured late. The headroom
/// covers only what is left after the page resolves: one round trip and the serialisation
/// of the reply.
pub const TRANSPORT_DEADLINE: std::time::Duration =
    std::time::Duration::from_millis(READY_CEILING_MS + 10_000);
/// The minimum spacing between geometry attempts, so a page that never stops moving rate
/// limits its own scans instead of running one per frame until the ceiling.
const RETRY_MS: u64 = 50;

/// Builds the page script that resolves once the page is settled.
///
/// `wait_for_lifecycle` additionally requires the lifecycle recorder to have closed its
/// window, and `wait_for_startup` requires any blocking overlay to have left.
/// Waits for the page to report itself settled, with the lifecycle recorder's window
/// included in that verdict.
pub async fn wait_ready(cdp: &mut crate::cdp::Cdp, wait_for_startup: bool) -> Result<()> {
    wait_ready_mode(cdp, wait_for_startup, true).await
}

/// The same wait for a page whose lifecycle window has already closed once, so requiring it
/// again would wait for a recorder that will never reopen.
pub async fn wait_ready_without_lifecycle(
    cdp: &mut crate::cdp::Cdp,
    wait_for_startup: bool,
) -> Result<()> {
    wait_ready_mode(cdp, wait_for_startup, false).await
}

/// A page that never reports itself settled is read anyway. "Settled" is a guess about a
/// page the tool has never seen, and failing here discarded every viewport already captured
/// over that guess, leaving nothing to audit it against. The page records the fact for
/// itself, so the artifact carries the doubt instead of the run carrying it as an error. A
/// transport failure is still a fact about the run and still fails.
async fn wait_ready_mode(
    cdp: &mut crate::cdp::Cdp,
    wait_for_startup: bool,
    wait_for_lifecycle: bool,
) -> Result<()> {
    cdp.evaluate(&source(wait_for_lifecycle, wait_for_startup))
        .await?;
    Ok(())
}

/// A settled capture that still holds a curtain probably recorded the splash screen rather
/// than the page — and "probably" is why this records the suspicion instead of raising it.
///
/// The verdict comes from a six-clause rule about a page the tool has never seen, so it is
/// a guess, and the two ways of being wrong cost wildly different amounts. Aborting threw
/// away every viewport already captured and wrote no files, which left nothing to audit and
/// so left the guess itself unfalsifiable. Reporting it costs a line. Facts about the run —
/// a lost transport, an unparseable response — still fail; heuristics about the page do not.
pub fn note_curtain(state: &mut crate::model::PageState) {
    let Some(path) = state
        .nodes
        .iter()
        .find(|node| node.blocking_overlay)
        .map(|node| node.path.clone())
    else {
        return;
    };
    let note = format!("settled capture still contains a blocking overlay at {path}");
    eprintln!("warning: {note}");
    state.capture_blockers.push(note);
}

pub fn source(wait_for_lifecycle: bool, wait_for_startup: bool) -> String {
    let lifecycle = if wait_for_lifecycle {
        " && window.__recreateLifecycleDone === true"
    } else {
        ""
    };
    format!(
        r#"(async () => {{
{settle}
  const blocking = {overlay};
  const networkQuiet = {network};
  const started = Date.now();
  const elapsed = () => Date.now() - started;
  let mutated = false;
  const observer = new MutationObserver(() => {{ mutated = true; }});
  observer.observe(document, {{
    subtree: true, childList: true, attributes: true, characterData: true
  }});
  const frame = () => new Promise(resolve => requestAnimationFrame(() => resolve()));
  const pause = () => new Promise(resolve => {timeout}(resolve, {RETRY_MS}));
  const ready = () => document.readyState === 'complete' &&
    document.fonts.status === 'loaded'{lifecycle};
  const scan = () => {{
    let shown = 0;
    let digest = 0;
    let curtain = false;
    const described = lifecycleDescribed(document);
    for (const element of document.querySelectorAll('*')) {{
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      const visible = rect.width > 0 && rect.height > 0 && style.display !== 'none' &&
        style.visibility !== 'hidden' && Number(style.opacity || 1) > 0;
      if (visible) {{
        const part = described.has(element)
          ? element.tagName + ':described'
          : [element.tagName, Math.round(rect.x), Math.round(rect.y),
            Math.round(rect.width), Math.round(rect.height), style.display].join(':');
        for (let index = 0; index < part.length; index++) {{
          digest = (Math.imul(digest, 31) + part.charCodeAt(index)) | 0;
        }}
        shown++;
      }}
      curtain ||= blocking(element);
    }}
    return {{ signature: shown ? shown + ':' + digest : '', curtain }};
  }};
  try {{
    while (elapsed() < {READY_CEILING_MS}) {{
      let quiet = 0;
      while (quiet < {QUIET_FRAMES} && elapsed() < {READY_CEILING_MS}) {{
        mutated = false;
        await frame();
        quiet = mutated || !networkQuiet() ? 0 : quiet + 1;
      }}
      if (quiet < {QUIET_FRAMES} || !ready()) {{ await pause(); continue; }}
      const before = scan();
      await frame();
      const after = scan();
      if (!after.signature) {{ await pause(); continue; }}
      if (before.signature !== after.signature && elapsed() < {STABLE_CEILING_MS}) {{
        await pause();
        continue;
      }}
      if ({wait_for_startup} && after.curtain) {{ await pause(); continue; }}
      return true;
    }}
    window.__recreateUnsettled = true;
    return false;
  }} finally {{
    observer.disconnect();
  }}
}})()"#,
        overlay = blocking_overlay::js_predicate(),
        network = crate::network_quiet::js_gate(),
        settle = crate::lifecycle_settle_script::SOURCE,
        timeout = crate::lifecycle_scheduled_script::INSTRUMENT_TIMEOUT,
    )
}

#[cfg(test)]
#[path = "capture_settle_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "capture_curtain_tests.rs"]
mod curtain_tests;
