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

/// How many consecutive animation frames must pass with no DOM edit before geometry is
/// worth reading. Two frames distinguish "nothing is happening" from "between two edits".
const QUIET_FRAMES: u32 = 2;
/// The longest a page may keep moving before it is captured mid-motion, for pages whose
/// geometry never repeats because something on them loops forever.
const STABLE_CEILING_MS: u64 = 8_000;
/// The longest a page may take to report itself loaded, or a startup curtain to leave,
/// before capture gives up entirely.
const READY_CEILING_MS: u64 = 30_000;
/// The minimum spacing between geometry attempts, so a page that never stops moving rate
/// limits its own scans instead of running one per frame until the ceiling.
const RETRY_MS: u64 = 50;

/// Builds the page script that resolves once the page is settled.
///
/// `wait_for_lifecycle` additionally requires the lifecycle recorder to have closed its
/// window, and `wait_for_startup` requires any blocking overlay to have left.
pub fn source(wait_for_lifecycle: bool, wait_for_startup: bool) -> String {
    let lifecycle = if wait_for_lifecycle {
        "window.__recreateLifecycleDone === true &&"
    } else {
        ""
    };
    format!(
        r#"(async () => {{
  const blocking = {overlay};
  const started = Date.now();
  const elapsed = () => Date.now() - started;
  let mutated = false;
  const observer = new MutationObserver(() => {{ mutated = true; }});
  observer.observe(document, {{
    subtree: true, childList: true, attributes: true, characterData: true
  }});
  const frame = () => new Promise(resolve => requestAnimationFrame(() => resolve()));
  const pause = () => new Promise(resolve => setTimeout(resolve, {RETRY_MS}));
  const ready = () => document.readyState === 'complete' &&
    document.fonts.status === 'loaded' && {lifecycle}
    (window.__recreatePendingRequests || 0) === 0;
  const scan = () => {{
    let shown = 0;
    let digest = 0;
    let curtain = false;
    for (const element of document.querySelectorAll('*')) {{
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      const visible = rect.width > 0 && rect.height > 0 && style.display !== 'none' &&
        style.visibility !== 'hidden' && Number(style.opacity || 1) > 0;
      if (visible) {{
        const part = [element.tagName, Math.round(rect.x), Math.round(rect.y),
          Math.round(rect.width), Math.round(rect.height), style.display].join(':');
        for (let index = 0; index < part.length; index++) {{
          digest = (Math.imul(digest, 31) + part.charCodeAt(index)) | 0;
        }}
        shown++;
      }}
      curtain ||= visible && blocking(element);
    }}
    return {{ signature: shown ? shown + ':' + digest : '', curtain }};
  }};
  try {{
    while (elapsed() < {READY_CEILING_MS}) {{
      let quiet = 0;
      while (quiet < {QUIET_FRAMES} && elapsed() < {READY_CEILING_MS}) {{
        mutated = false;
        await frame();
        quiet = mutated ? 0 : quiet + 1;
      }}
      if (!ready()) {{ await pause(); continue; }}
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
    return false;
  }} finally {{
    observer.disconnect();
  }}
}})()"#,
        overlay = blocking_overlay::js_predicate(),
    )
}

#[cfg(test)]
#[path = "capture_settle_tests.rs"]
mod tests;
