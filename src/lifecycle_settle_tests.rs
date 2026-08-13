use crate::node_eval;

/// One call of the shipped predicate: elapsed, time since the last change, whether anything
/// is still running, and the longest gap in motion this page has already recovered from.
struct Case {
    elapsed: u64,
    since_change: u64,
    busy: bool,
    longest_gap: u64,
}

fn case(elapsed: u64, since_change: u64, busy: bool, longest_gap: u64) -> Case {
    Case {
        elapsed,
        since_change,
        busy,
        longest_gap,
    }
}

/// Evaluates the real predicate under Node, so the assertions below constrain the shipped
/// decision rather than the words it is written with.
fn settled(cases: &[Case]) -> Vec<bool> {
    let calls = cases
        .iter()
        .map(|entry| {
            format!(
                "lifecycleSettled({}, {}, {}, {})",
                entry.elapsed, entry.since_change, entry.busy, entry.longest_gap
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    node_eval::evaluate(super::SOURCE, &format!("[{calls}]"))
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_bool().unwrap())
        .collect()
}
/// The cost of a capture must come from the page, not from a constant. A page that never
/// moved has shown no gap it can recover from, so one quiet frame is the whole wait — the
/// case that used to be charged a full second.
#[test]
fn a_page_that_never_moved_releases_the_recorder_after_a_single_frame() {
    assert_eq!(settled(&[case(16, 16, false, 0)]), vec![true]);
}

/// The inverse, and the reason the gap is measured at all. A page that has already paused
/// for 400ms and resumed must not be released during a 400ms pause, however long the
/// recorder has been running overall.
#[test]
fn a_pause_no_longer_than_one_the_page_recovered_from_holds_the_recorder_open() {
    let results = settled(&[
        case(5_000, 399, false, 400),
        case(5_000, 400, false, 400),
        case(5_000, 401, false, 400),
    ]);
    assert_eq!(results, vec![false, false, true]);
}

/// Direct evidence outranks inference: anything still running or still loading holds the
/// recorder open no matter how quiet the page has gone.
#[test]
fn running_motion_and_pending_loads_hold_the_recorder_open() {
    let results = settled(&[case(5_000, 5_000, true, 0), case(11_999, 11_999, true, 0)]);
    assert_eq!(results, vec![false, false]);
}

/// A page that never stops moving must still be captured, so the horizon stays a ceiling
/// and outranks every other reason to keep waiting.
#[test]
fn a_page_that_never_settles_is_released_at_the_ceiling() {
    let results = settled(&[case(12_000, 0, true, 5_000), case(11_999, 0, true, 5_000)]);
    assert_eq!(results, vec![true, false]);
}

/// The gap is a floor on the wait and never a reason to settle sooner, so a longer observed
/// gap can only ever delay release.
#[test]
fn a_longer_observed_gap_never_releases_the_recorder_earlier() {
    let mut previous = true;
    for gap in [0_u64, 100, 500, 2_000] {
        let current = settled(&[case(5_000, 500, false, gap)])[0];
        assert!(
            !current || previous,
            "a {gap}ms gap released the recorder that a shorter gap held open"
        );
        previous = current;
    }
}

fn awaited(soonest_due: &str, start: f64, changed: bool) -> bool {
    node_eval::evaluate(
        super::SOURCE,
        &format!("lifecycleAwaited({soonest_due}, {start}, {changed})"),
    )
    .as_bool()
    .unwrap()
}

/// Before its first edit a page has shown no gap it can recover from, so one quiet frame
/// satisfies the settle rule and the window shuts before anything the page scheduled has
/// run. Work the page has scheduled is the evidence that closes that hole, and it must be
/// awaited on exactly the same footing as a request in flight.
#[test]
fn work_the_page_scheduled_before_the_ceiling_is_awaited() {
    assert!(awaited("300", 0.0, false));
    assert!(awaited("11999", 0.0, false));
}

/// Work due after the recorder's own horizon cannot produce anything the recorder will
/// see, so waiting for it buys nothing and costs the whole ceiling. This is what keeps a
/// page carrying a long poll timer from being watched for twelve seconds.
#[test]
fn work_due_after_the_ceiling_is_not_worth_waiting_for() {
    assert!(!awaited("12001", 0.0, false));
    assert!(!awaited("30000", 0.0, false));
}

/// The horizon is measured from when the recorder started, not from now, so a timer stays
/// worth waiting for as the recorder runs rather than becoming so.
#[test]
fn the_horizon_is_anchored_to_the_start_of_the_recording() {
    assert!(awaited("11000", 0.0, false));
    assert!(!awaited("11000", -2_000.0, false));
}

/// A page that has scheduled nothing reports no soonest due time at all, and must not be
/// held open by its absence.
#[test]
fn a_page_that_scheduled_nothing_is_never_awaited() {
    assert!(!awaited("Infinity", 0.0, false));
}

/// The blind spot the schedule was admitted to cover is named precisely in
/// `lifecycle_scheduled_script`: the silence before the page's first edit, where the longest
/// observed gap is still zero and one frame of quiet closes the window. Once the page has
/// edited itself it has demonstrated its own cadence, and the doc says the observed-gap rule
/// takes over from there.
///
/// It did not. The horizon was compared against the ceiling alone, so a timer the page set up
/// front for eight or ten seconds — a heartbeat, a deferred banner, a carousel advance, all
/// routine on a production page — held the recorder open until it fired, no matter how much
/// the page had already shown. That is the ceiling becoming the routine path again, one stage
/// further along: a fixture schedules nothing and never notices, while a live page is held
/// for a timer that has already been superseded by evidence the recorder trusts more.
#[test]
fn a_schedule_stops_being_evidence_once_the_page_has_edited_itself() {
    assert!(
        awaited("8000", 0.0, false),
        "a page that has not yet moved must still be held by what it scheduled"
    );
    assert!(
        !awaited("8000", 0.0, true),
        "a page that has already edited itself was still held open by a timer"
    );
}
