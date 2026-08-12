use super::*;
use crate::generate::before_change::BeforeChange;
use serde_json::json;

fn scripted(keyframes: Vec<serde_json::Value>) -> Animation {
    Animation {
        target: "html>body>div".into(),
        name: String::new(),
        keyframes,
        timing: json!({"duration": 4000, "iterations": 1, "fill": "forwards"}),
    }
}

fn emit(animation: Animation) -> String {
    let mut classes = BTreeMap::from([("html>body>div".into(), "base".into())]);
    let mut css = String::new();
    append(
        &[animation],
        &BTreeSet::new(),
        &BeforeChange::default(),
        &mut classes,
        &mut css,
    );
    css
}

/// `getKeyframes()` reports `offset` only where the author stated one and `computedOffset`
/// for every frame, spacing unanchored frames between their nearest anchored neighbours
/// rather than evenly across the whole list. Here the browser puts the second frame at
/// 0.35, between 0 and the anchor at 0.7; spacing by index would put it at 1/3.
#[test]
fn places_a_frame_at_the_position_the_browser_recorded() {
    let css = emit(scripted(vec![
        json!({"computedOffset":0.0,"offset":null,"opacity":"0"}),
        json!({"computedOffset":0.35,"offset":null,"opacity":"0.4"}),
        json!({"computedOffset":0.7,"offset":0.7,"opacity":"0.6"}),
        json!({"computedOffset":1.0,"offset":null,"opacity":"1"}),
    ]));
    assert!(css.contains("35%{opacity:0.4;}"), "{css}");
    assert!(
        !css.contains("33%{"),
        "spaced by index rather than by record: {css}"
    );
}

/// Positions are rounded to a whole percentage and used as a map key whose entries merge,
/// so a guessed position that lands on another frame's recorded one does not merely
/// mistime a frame — it overwrites it, and the authored declaration is emitted nowhere.
/// Spacing by index puts the second frame at 1/3, colliding with the anchor at 0.33.
#[test]
fn keeps_a_frame_whose_guessed_position_would_collide_with_a_recorded_one() {
    let css = emit(scripted(vec![
        json!({"computedOffset":0.0,"offset":null,"opacity":"0.11"}),
        json!({"computedOffset":0.165,"offset":null,"opacity":"0.37"}),
        json!({"computedOffset":0.33,"offset":0.33,"opacity":"0.53"}),
        json!({"computedOffset":1.0,"offset":null,"opacity":"0.79"}),
    ]));
    assert_eq!(
        css.matches("%{").count(),
        4,
        "lost an authored frame: {css}"
    );
    assert!(css.contains("opacity:0.37"), "{css}");
    assert!(css.contains("33%{opacity:0.53;}"), "{css}");
}

/// The index guess stays as a last resort so a record carrying neither field still emits.
/// It is what the browser computes only when no interior frame is anchored, which is
/// exactly the case a record without positions cannot contradict.
#[test]
fn spaces_frames_evenly_when_no_position_was_recorded() {
    let css = emit(scripted(vec![
        json!({"opacity":"0"}),
        json!({"opacity":"0.5"}),
        json!({"opacity":"1"}),
    ]));
    assert!(css.contains("0%{opacity:0;}"), "{css}");
    assert!(css.contains("50%{opacity:0.5;}"), "{css}");
    assert!(css.contains("100%{opacity:1;}"), "{css}");
}

/// An explicit `offset` is honoured on records that predate `computedOffset`, so the
/// chain degrades one step at a time rather than falling straight through to the guess.
#[test]
fn honours_an_explicit_offset_when_no_computed_one_was_recorded() {
    let css = emit(scripted(vec![
        json!({"offset":0.0,"opacity":"0"}),
        json!({"offset":0.8,"opacity":"0.5"}),
        json!({"offset":1.0,"opacity":"1"}),
    ]));
    assert!(css.contains("80%{opacity:0.5;}"), "{css}");
}
