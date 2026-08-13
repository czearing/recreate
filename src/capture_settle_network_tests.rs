//! Tests for the gate the settle loop applies before it will read a page at all: the page
//! must report itself ready, it must have something visible to read, and its network must be
//! quiet enough. Each case drives the shipped loop under Node against a scripted page, so it
//! constrains the decision that ships rather than a Rust restatement of it.

use super::*;

/// Readiness and lifecycle are separate gates from stillness, and a page that never opens
/// them must be reported as unsettled rather than captured.
#[test]
fn a_page_that_never_reports_ready_is_not_settled() {
    let loading = json!({ "steps": [{ "elements": [box_at(0.0)], "loading": true }] });
    assert!(!resolved(&settle(loading, true)));

    let fonts = json!({ "steps": [{ "elements": [box_at(0.0)], "fontsPending": true }] });
    assert!(!resolved(&settle(fonts, true)));

    let lifecycle = json!({ "steps": [still()], "lifecyclePending": true });
    assert!(!resolved(&settle(lifecycle, true)));
}

/// A page with nothing visible on it has no geometry to prove still, so it must not be
/// treated as settled on the strength of an empty reading.
#[test]
fn a_page_with_nothing_visible_is_not_settled() {
    let empty = json!({ "steps": [{ "elements": [] }] });
    assert!(!resolved(&settle(empty, true)));
}

/// A page served from disk finishes every request before the first frame, so demanding zero
/// in flight cost a fixture nothing and looked correct for as long as fixtures were the only
/// input. A production page holds a permanent floor of telemetry, long-poll and open streams,
/// so that demand is unsatisfiable by construction: the loop does not fail, it spins to the
/// ready ceiling, and the constant rather than the page decides how long capture takes. The
/// bound here is far below that ceiling, so any return to zero-in-flight fails this outright.
#[test]
fn a_page_holding_a_permanent_floor_of_requests_still_settles() {
    let result = settle(json!({ "steps": [with_pending(still(), 2)] }), true);
    assert!(
        resolved(&result),
        "a page with two requests permanently in flight never settled"
    );
    assert!(
        result["elapsed"].as_u64().unwrap() < 2_000,
        "a page already still behind a request floor took {}ms",
        result["elapsed"]
    );
}

/// The tolerance is an allowance for a floor, not permission to ignore the network. A page
/// still pulling down more than the tolerated count is loading, and reading it would record
/// a half-built page under a clean result.
#[test]
fn a_page_still_fetching_beyond_the_tolerance_is_not_settled() {
    let busy = json!({ "steps": [with_pending(still(), 5)] });
    assert!(!resolved(&settle(busy, true)));
}

/// What the tolerance lets through, and the check that covers it. Tolerating a request means
/// accepting that it may be the one carrying visible content, so the DOM window has to keep
/// answering for the network the moment the network stops answering for itself. A page whose
/// tolerated requests are still editing it must be held back for exactly as long as a page
/// with nothing in flight would be, or the tolerance has quietly disabled the other signal.
#[test]
fn edits_still_hold_back_a_page_behind_a_request_floor() {
    let mut steps: Vec<Value> = (0..45)
        .map(|_| with_pending(json!({ "elements": [box_at(0.0)], "mutate": true }), 2))
        .collect();
    steps.push(with_pending(still(), 2));
    let result = settle(json!({ "steps": steps }), true);
    assert!(resolved(&result));
    assert!(
        frames(&result) >= 45,
        "captured after {} frames, while tolerated requests were still editing the page",
        frames(&result)
    );
}
fn with_pending(mut step: Value, pending: u64) -> Value {
    step["pending"] = json!(pending);
    step
}
