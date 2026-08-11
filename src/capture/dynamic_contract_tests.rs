//! Contracts between the observation script and the modules whose rules it carries. Kept
//! apart from the timing tests because these assert wiring rather than behaviour: they fail
//! when a rule stops being reached, which no measurement of a released page can show.

/// The whole reason this module no longer decides for itself: the rule it applies is the one
/// `lifecycle_settle_script` owns, inlined rather than restated, so the two cannot drift.
#[test]
fn the_script_carries_the_owning_modules_rule_rather_than_a_copy() {
    let source = super::source();
    assert!(source.contains(crate::lifecycle_settle_script::SOURCE));
    assert!(!source.contains("__LIFECYCLE_SETTLE__"));
    assert!(source.contains("lifecycleSettled("));
}

/// The defect that made every timing test unobservable. This module and its whole rule were
/// reachable only from `capture_state_with_startup`, which the default capture never calls,
/// so no capture ever watched a page for a value cycle: sequences survived only when some
/// other stage happened to wait long enough. A page driven by `setInterval` has no such
/// stage — its schedule stops being evidence at the first tick — so its loop was recorded as
/// three values and emitted as a progression that comes to rest.
///
/// The observation belongs to a fresh load, because a timed progression happens once from
/// one, so `prepare_state` runs it under exactly the flag that says the page was reloaded.
#[test]
fn every_freshly_loaded_state_is_watched_for_value_cycles() {
    let source = include_str!("state.rs");
    assert!(
        source.contains("observe_dynamic(cdp, reload)"),
        "a freshly loaded state is read without watching it for value cycles"
    );
    assert!(
        source.contains("super::dynamic::observe(cdp)"),
        "the observation no longer reaches the module that owns it"
    );
    assert!(
        !source.contains("observe_dynamic: bool"),
        "the caller decides again whether to observe, which is how it was lost"
    );
}

/// The margin an unfinished progression is given lives in one place: the gate's own idea of
/// the shortest gap worth keeping. Waiting less than that could only end a progression the
/// gate would discard anyway, so no second constant is needed and none may appear.
#[test]
fn the_observer_waits_the_gate_s_own_shortest_cadence_rather_than_a_copy_of_it() {
    let source = super::source();
    assert!(!source.contains("__STABLE_GAP_MS__"));
    assert!(source.contains(&format!(
        "longestGap + {}",
        crate::attribute_sequence_script::STABLE_GAP_MS
    )));
}

/// Grouping must stay identical to what the sequence capture applies afterwards, or the
/// recorder would judge a cycle proven that the consumer then reads as unfinished.
#[test]
fn the_reading_groups_changes_the_way_sequence_capture_does() {
    let source = super::source();
    assert!(source.contains("__recreateAttributeMutations"));
    assert!(source.contains("values.at(-1) !== event.value"));
    assert!(source.contains("value === values[index % size]"));
    assert!(source.contains("values.length < 3 || cycle === values.length"));
}
