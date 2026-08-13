use crate::node_eval;
use serde_json::Value;

/// Drives the shipped tracker against a fake scope whose timers are handles this test hands
/// out itself, so the rule is constrained without waiting for real time to pass.
fn track(body: &str) -> Value {
    node_eval::evaluate(
        &format!(
            r#"{source}
const scope = {{
  performance: {{ now: () => scope.clock }},
  clock: 0,
  next: 1,
  fired: [],
  setTimeout(handler, delay) {{
    const handle = scope.next++;
    scope.queue.set(handle, handler);
    return handle;
  }},
  setInterval(handler, delay) {{
    const handle = scope.next++;
    scope.queue.set(handle, handler);
    return handle;
  }},
  clearTimeout() {{}},
  clearInterval() {{}},
  queue: new Map()
}};
const soonest = trackScheduled(scope);
const fire = handle => scope.queue.get(handle)();
"#,
            source = super::SOURCE
        ),
        body,
    )
}

/// The defect this rule exists to remove. Before the page's first edit the recorder has
/// measured nothing, so a single quiet frame satisfies it and the window shuts before the
/// first scheduled change has run. A timer that has been scheduled and not yet fired is the
/// page stating that more is to come.
#[test]
fn work_the_page_has_scheduled_is_outstanding_until_it_runs() {
    assert_eq!(
        track(
            "(() => { scope.clock = 100; const h = scope.setTimeout(() => {}, 300); return soonest(); })()"
        ),
        400.0
    );
}

/// Evidence expires as it is consumed, or the recorder waits out a change that has already
/// happened.
#[test]
fn a_timer_stops_counting_once_it_has_fired() {
    assert_eq!(
        track(
            "(() => { const h = scope.setTimeout(() => {}, 300); fire(h); return soonest(); })()"
        ),
        Value::Null
    );
}

/// Cancelled work will never arrive, so it is not evidence either. Without this a page that
/// schedules and cancels is watched to the ceiling for a change that cannot come.
#[test]
fn cancelled_work_stops_counting_immediately() {
    assert_eq!(
        track(
            "(() => { const h = scope.setTimeout(() => {}, 300); scope.clearTimeout(h); return soonest(); })()"
        ),
        Value::Null
    );
}

/// A repeating interval is a periodic process, and one period describes it completely. It
/// counts until its first tick and no further, which is what stops a page carrying a
/// perpetual interval from being watched to the ceiling.
#[test]
fn a_repeating_interval_counts_only_until_its_first_tick() {
    assert_eq!(
        track(
            "(() => { const h = scope.setInterval(() => {}, 300); fire(h); fire(h); return soonest(); })()"
        ),
        Value::Null
    );
}

/// The recorder waits for the change that arrives first, not the last one scheduled.
#[test]
fn the_soonest_outstanding_work_is_the_one_reported() {
    assert_eq!(
        track(
            "(() => { scope.setTimeout(() => {}, 900); scope.setTimeout(() => {}, 300); return soonest(); })()"
        ),
        300.0
    );
}

/// A page with nothing scheduled must cost nothing, so the absence of work has to be
/// reportable as a time that no ceiling can precede.
#[test]
fn a_page_that_schedules_nothing_reports_no_outstanding_work() {
    assert_eq!(track("soonest() === Infinity"), Value::Bool(true));
}

/// The clause the rule turns on. A poll loop has one timer outstanding at every instant and
/// never a last one, so admitting its renewals would hold the recorder open for as long as
/// the loop runs. Measured against a browser extension's poll loop on a page with no script
/// of its own: 300 of 301 frames reported busy, 5.6s per capture.
#[test]
fn a_timer_that_reschedules_itself_stops_counting_after_its_first_tick() {
    assert_eq!(
        track(
            "(() => { const tick = () => { scope.setTimeout(tick, 50); }; \
             const h = scope.setTimeout(tick, 50); fire(h); return soonest(); })()"
        ),
        Value::Null
    );
}

/// The loop that cost 5.6s reschedules from a promise continuation, so the callback has
/// already returned by the time the next timer is asked for. Nothing about the shape of the
/// call distinguishes it, which is why the rule turns on the schedule having started rather
/// than on the renewal being nested inside its predecessor.
#[test]
fn a_loop_that_reschedules_after_its_callback_returns_is_still_a_renewal() {
    assert_eq!(
        track(
            "(() => { const h = scope.setTimeout(() => {}, 50); fire(h); \
             scope.setTimeout(() => {}, 50); return soonest(); })()"
        ),
        Value::Null
    );
}

/// The narrowing must not swallow the case the rule exists for: a page that schedules its
/// whole progression up front is still stating that more is to come, and every step of it
/// counts until it runs — including the steps still queued behind the first tick.
#[test]
fn a_progression_scheduled_up_front_counts_through_to_its_last_step() {
    assert_eq!(
        track(
            "(() => { const first = scope.setTimeout(() => {}, 100); scope.setTimeout(() => {}, 400); \
             fire(first); return soonest(); })()"
        ),
        400.0
    );
}

/// Wrapping the page's own timers must not change what the page observes through them.
#[test]
fn the_page_still_receives_its_handle_and_its_arguments() {
    assert_eq!(
        track(
            "(() => { const seen = []; const h = scope.setTimeout((...args) => seen.push(args.length), 0); \
             fire(h); return [typeof h, seen]; })()"
        ),
        serde_json::json!(["number", [0]])
    );
}

/// Every script the harness runs inside the page it is measuring, so that no new one can be
/// added through the wrapped scheduler without this failing.
fn instrument_sources() -> Vec<(&'static str, String)> {
    vec![
        ("capture_settle", crate::capture_settle::source(true, true)),
        (
            "capture_settle bare",
            crate::capture_settle::source(false, false),
        ),
        ("first_paint", crate::first_paint::source()),
    ]
}

/// An instrument must not appear in its own measurement. These scripts poll the page from
/// inside it, so a bare `setTimeout` in one of them is read as work the page still owes and
/// holds the recorder open until the instrument stops — which it only does once the recorder
/// closes. That deadlock measured as a 4702ms recorder window against 0.15s for the
/// viewports the instrument had not reached.
#[test]
fn no_capture_instrument_schedules_through_the_wrapped_scheduler() {
    for (name, source) in instrument_sources() {
        for line in source.lines() {
            let scheduling = line
                .split("setTimeout")
                .skip(1)
                .any(|rest| rest.trim_start().starts_with('('));
            let unwrapped = scheduling && !line.contains(super::INSTRUMENT_TIMEOUT);
            assert!(
                !unwrapped,
                "{name} schedules through the page's own scheduler: {}",
                line.trim()
            );
        }
    }
}

/// The instruments must actually reach the scheduler the tracker set aside, so the test
/// above cannot pass by an instrument having stopped scheduling at all.
///
/// The rule binds an instrument that schedules. One that does not cannot reach the page's
/// scheduler and so cannot appear in its own measurement — the reader that records first
/// paint waits on an animation frame, which the tracker does not wrap and the recorder does
/// not count as work the page still owes. The non-vacuity the doc above is protecting is
/// therefore asserted over the set rather than over each member, which is where it belongs:
/// per-member it would have forbidden a timer-free instrument from existing.
#[test]
fn every_instrument_that_waits_reaches_the_unmeasured_scheduler() {
    let mut waiting = 0;
    for (name, source) in instrument_sources() {
        if !source.contains("setTimeout") {
            continue;
        }
        waiting += 1;
        assert!(
            source.contains(super::INSTRUMENT_TIMEOUT),
            "{name} no longer waits through the unmeasured scheduler"
        );
    }
    assert!(
        waiting > 0,
        "no instrument schedules any more, so the rule above is unenforced"
    );
}
