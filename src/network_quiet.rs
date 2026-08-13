//! The single owner of "is the network quiet enough to read this page?".
//!
//! A page served from disk finishes every request it will ever make before the first frame,
//! so demanding zero in flight cost a fixture nothing and looked correct for as long as
//! fixtures were the only input. A production page never reaches zero: telemetry beacons,
//! long-poll and open streams are a floor rather than a transient, so the demand is
//! unsatisfiable by construction and the loop waiting on it is decided by its own ceiling
//! instead of by the page. Chromium reports `networkAlmostIdle` at two or fewer connections,
//! Lighthouse gates on the same number, and Puppeteer documents the zero form as able to
//! hang indefinitely on production.
//!
//! Tolerating requests would only move the question, because the tolerated request may be
//! the one carrying the content, so the tolerance is paired with a hold: a count within
//! tolerance must survive `HOLD_MS` before it is believed. A page with nothing in flight has
//! nothing outstanding that could arrive, so it skips the hold and settles as immediately as
//! it always did — which is why no fixture can observe this rule changing.

/// The most requests a page may hold in flight and still be called quiet.
const TOLERANCE: u32 = 2;
/// How long a tolerated but non-zero count must hold before it is believed.
const HOLD_MS: u64 = 500;

/// Builds a JS predicate that answers whether the network has been quiet enough for long
/// enough. Each call site gets its own, because the hold is measured from the last moment the
/// count was above tolerance, and that is state. It must be polled once per animation frame:
/// a gate asked only at the end of a window cannot know the window was quiet throughout it.
pub fn js_gate() -> String {
    format!(
        r#"(() => {{
  let since = Date.now();
  return () => {{
    const pending = window.__recreatePendingRequests || 0;
    if (pending > {TOLERANCE}) {{
      since = Date.now();
      return false;
    }}
    return pending === 0 || Date.now() - since >= {HOLD_MS};
  }};
}})()"#
    )
}
