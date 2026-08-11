//! Scripted timelines for the observation script, driven on a virtual clock so a scenario
//! spanning the full ceiling runs in microtasks with no browser and no real elapsed time.

use crate::node_eval;
use serde_json::{Value, json};

pub(super) const HARNESS: &str = include_str!("dynamic_harness.js");
/// The horizon the settle rule owns, past which any page is released.
pub(super) const CEILING_MS: u64 = 12_000;
/// One virtual animation frame, matching the harness clock.
pub(super) const FRAME_MS: u64 = 16;

/// One attribute change on one element, as the lifecycle recorder writes it.
pub(super) fn change(target: &str, value: &str) -> Value {
    json!({ "target": target, "attribute": "title", "value": value })
}

/// A change the recorder had already written before the observer took over, stamped on the
/// recorder's clock.
pub(super) fn recorded(target: &str, value: &str, time: u64) -> Value {
    json!({ "target": target, "attribute": "title", "value": value, "time": time })
}

/// A frame that appends nothing, so the page is simply still.
pub(super) fn quiet(count: usize) -> Vec<Vec<Value>> {
    vec![vec![]; count]
}

/// Runs the real shipped script over a scripted timeline and reports the virtual
/// milliseconds it watched the page for before letting go.
pub(super) fn observed(scene: Vec<Vec<Value>>) -> u64 {
    observed_after(Vec::new(), scene)
}

/// The same, for a page the recorder had already been watching when the observer attached.
pub(super) fn observed_after(history: Vec<Value>, scene: Vec<Vec<Value>>) -> u64 {
    let script = HARNESS
        .replace("__SCENE__", &serde_json::to_string(&scene).unwrap())
        .replace("__HISTORY__", &serde_json::to_string(&history).unwrap())
        .replace("__SCRIPT__", super::source().trim());
    node_eval::json(&script)["elapsed"].as_u64().unwrap()
}
