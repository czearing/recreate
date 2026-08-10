use super::Timeline;
use crate::generate::{jsx_variants, startup_overlays, tree};
use crate::model::PageState;

/// Deliberately unequal, so an emitter that writes one phase twice cannot pass.
const DELAY_MS: u64 = 1200;
const DURATION_MS: u64 = 800;

fn state(delay_ms: u64, duration_ms: u64) -> PageState {
    PageState {
        startup_delay_ms: delay_ms,
        startup_duration_ms: duration_ms,
        ..crate::generate::project_test_support::state(1280)
    }
}

fn components() -> tree::Components {
    tree::Components {
        items: Vec::new(),
        by_root: Default::default(),
        children: Default::default(),
        classes: Default::default(),
        nodes: Default::default(),
    }
}

/// Capture measures a pre-curtain phase and a curtain phase. Both are facts about what the
/// page did, so both have to reach the recreation or its timeline is a different one.
#[test]
fn carries_both_captured_phases() {
    let variables = Timeline::of(&state(DELAY_MS, DURATION_MS)).style_variables();
    assert_eq!(
        variables,
        r#""--recreate-startup-delay":"1200ms","--recreate-startup-duration":"800ms""#
    );
}

/// The relation, not the constant: the settle instant must move exactly as far as the span
/// moves. An emitter that timed off one phase alone would break this for some pair.
#[test]
fn the_settle_instant_follows_the_whole_span() {
    let short = Timeline::of(&state(DELAY_MS, DURATION_MS));
    let long = Timeline::of(&state(DELAY_MS + 400, DURATION_MS + 300));
    assert_eq!(long.settle_ms() - short.settle_ms(), 700);
    assert!(short.settle_ms() > DELAY_MS + DURATION_MS);
}

/// A page with no curtain has no startup layer, and must not be made to wait for one.
#[test]
fn a_page_without_a_curtain_has_no_settle_instant() {
    assert_eq!(Timeline::of(&state(0, 0)).settle_ms(), 0);
}

/// A curtain first seen on the final poll has a real delay and an unmeasurably short stay.
/// Timing off the duration alone would treat this page as having had no startup at all.
#[test]
fn a_delay_with_an_immeasurable_curtain_still_settles() {
    assert!(Timeline::of(&state(900, 0)).settle_ms() > 900);
}

/// The baked fragment is where the delay was replaced by a literal, so assert the captured
/// value reaches it rather than merely that the property is present.
#[test]
fn the_emitted_fragment_carries_the_captured_delay() {
    let fragment = jsx_variants::fragment(
        &components(),
        &Default::default(),
        Timeline::of(&state(DELAY_MS, DURATION_MS)),
    );
    assert!(
        fragment.contains(r#""--recreate-startup-delay":"1200ms""#),
        "fragment discarded the captured delay: {fragment}"
    );
    assert!(fragment.contains(r#""--recreate-startup-duration":"800ms""#));
}

/// The CSS animation and the JS timer replay one timeline. If the timer counts only the
/// curtain, it releases the body scroll-lock while the curtain is still fading.
#[test]
fn the_runtime_timer_waits_for_the_whole_span() {
    let source = startup_overlays::runtime(
        "const[state,setState]=useState(0);const activate=".into(),
        &[state(DELAY_MS, DURATION_MS)],
    );
    let settle = Timeline::of(&state(DELAY_MS, DURATION_MS)).settle_ms();
    assert!(
        source.contains(&format!("[{settle}]")),
        "runtime timer ignored the delay: {source}"
    );
}
