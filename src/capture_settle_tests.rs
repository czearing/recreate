use crate::node_eval;
use serde_json::{Value, json};

const HARNESS: &str = concat!(
    include_str!("dom_style_harness.js"),
    include_str!("capture_settle_harness.js")
);

/// Runs the shipped settle script against a scripted page and reports what it decided.
fn settle(scene: Value, wait_for_startup: bool) -> Value {
    node_eval::json(
        &HARNESS
            .replace("__SCENE__", &scene.to_string())
            .replace("__SETTLE__", &super::source(true, wait_for_startup)),
    )
}

fn box_at(x: f64) -> Value {
    json!({ "rect": { "x": x, "y": 0.0, "width": 100.0, "height": 100.0 } })
}

fn curtain() -> Value {
    json!({
        "rect": { "x": 0.0, "y": 0.0, "width": 1000.0, "height": 1000.0 },
        "style": { "position": "fixed", "z-index": "100" }
    })
}

fn still() -> Value {
    json!({ "elements": [box_at(0.0)] })
}

fn resolved(result: &Value) -> bool {
    result["resolved"].as_bool().unwrap()
}

fn frames(result: &Value) -> u64 {
    result["frames"].as_u64().unwrap()
}

/// The whole point of the rewrite. A page that is already still must be captured as soon as
/// it can be shown to be still, which is a few animation frames, not a poll interval. The
/// bound is deliberately far below the 250ms tick the old loop could not go under: any
/// return to interval polling fails this outright.
#[test]
fn a_page_that_is_already_still_settles_within_a_few_frames() {
    let result = settle(json!({ "steps": [still()] }), true);
    assert!(resolved(&result));
    assert!(
        result["elapsed"].as_u64().unwrap() < 200,
        "settling a still page took {}ms",
        result["elapsed"]
    );
}

/// Motion a stylesheet declares never stops, so demanding geometric stillness of a page
/// carrying a perpetual authored animation means waiting for it to happen to pause at a
/// turning point — a wait with no upper bound short of the ceiling, and one that varied by
/// seconds between runs of the same page. The animation is already recorded where the
/// capture reads it, so its target's movement is not evidence the page is unfinished.
#[test]
fn a_page_whose_only_motion_is_a_declared_animation_settles_at_once() {
    let result = settle(perpetual_animation(true), true);
    assert!(resolved(&result));
    assert!(
        result["elapsed"].as_u64().unwrap() < 200,
        "a declared animation held capture for {}ms",
        result["elapsed"]
    );
}

/// The inverse, and the reason this is a statement about where motion is recorded rather
/// than a licence to ignore anything animated. A script-built animation still part-way
/// through its first period is motion nothing has written down, so it must hold capture.
#[test]
fn a_script_built_animation_still_in_its_first_period_holds_capture_back() {
    let result = settle(perpetual_animation(false), true);
    assert!(
        result["elapsed"].as_u64().unwrap() >= 8_000,
        "undeclared motion released capture after {}ms",
        result["elapsed"]
    );
}

/// A page holding one still element beside one that moves under an animation forever, so
/// only where that animation is recorded can decide whether capture may proceed.
fn perpetual_animation(declared: bool) -> Value {
    let step = |index: i32| {
        json!({
            "elements": [box_at(0.0), box_at(f64::from(index) * 3.0)],
            "animations": [
                { "element": 1, "declared": declared, "duration": 4000, "localTime": 10 }
            ]
        })
    };
    json!({ "steps": (0..4000).map(step).collect::<Vec<_>>() })
}

/// Quiet has to be observed, not assumed. Every DOM edit restarts the window, so a page
/// still building itself cannot be captured half-built no matter how cheap the check is.
#[test]
fn dom_edits_restart_the_quiet_window() {
    let mut steps: Vec<Value> = (0..12)
        .map(|_| json!({ "elements": [box_at(0.0)], "mutate": true }))
        .collect();
    steps.push(still());
    let result = settle(json!({ "steps": steps }), true);
    assert!(resolved(&result));
    assert!(
        frames(&result) >= 12,
        "captured after {} frames while the page was still being edited",
        frames(&result)
    );
}

/// A CSS transition moves layout without touching the DOM, so mutation quiet alone would
/// report a page settled while it was still sliding. The geometry signature is what covers
/// that case, and removing it must fail here.
#[test]
fn layout_that_moves_without_a_dom_edit_is_not_settled() {
    let mut steps: Vec<Value> = (0..30)
        .map(|index| json!({ "elements": [box_at(f64::from(index) * 10.0)] }))
        .collect();
    steps.push(still());
    let result = settle(json!({ "steps": steps }), true);
    assert!(resolved(&result));
    assert!(
        frames(&result) >= 30,
        "captured after {} frames while layout was still moving",
        frames(&result)
    );
}

/// A startup curtain hides the page behind it, so capturing through one records the splash
/// screen instead of the site.
#[test]
fn a_blocking_curtain_holds_capture_back_until_it_leaves() {
    let mut steps: Vec<Value> = (0..15)
        .map(|_| json!({ "elements": [box_at(0.0), curtain()] }))
        .collect();
    steps.push(still());
    let waited = settle(json!({ "steps": steps.clone() }), true);
    assert!(resolved(&waited));
    assert!(
        frames(&waited) >= 15,
        "captured through the curtain after {} frames",
        frames(&waited)
    );

    // The inverse: a caller that is not waiting out startup must not be held by the same
    // element, or every capture of a page with a fixed full-bleed header would stall.
    let ignored = settle(json!({ "steps": steps }), false);
    assert!(resolved(&ignored));
    assert!(
        frames(&ignored) < 15,
        "a caller not waiting for startup still waited {} frames",
        frames(&ignored)
    );
}

/// A page running a permanent animation never repeats its geometry, so the ceiling has to
/// release it — and it must not spin one scan per frame while it waits.
#[test]
fn a_page_that_never_stops_moving_is_released_at_the_ceiling() {
    let forever = json!({
        "steps": (0..4000)
            .map(|index| json!({ "elements": [box_at(f64::from(index) * 3.0)] }))
            .collect::<Vec<_>>()
    });
    let result = settle(forever, true);
    assert!(resolved(&result), "the ceiling never released the page");
    assert!(
        result["elapsed"].as_u64().unwrap() >= 8_000,
        "released before the stability ceiling at {}ms",
        result["elapsed"]
    );
}

#[path = "capture_settle_network_tests.rs"]
mod network;
