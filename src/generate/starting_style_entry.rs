//! The entry motion an authored `@starting-style` describes, built from what the author wrote.
//!
//! A transition that runs on an element's first render is over in a few hundred milliseconds
//! and a capture that is not looking at that moment sees nothing of it. Recording it therefore
//! used to depend on the capture happening to read the page while the motion was still in the
//! air, which is not a property of the page — it is a property of how fast the machine was, and
//! it made the entry motion of the same page present in one run and absent in the next.
//!
//! The page states the whole thing statically. `@starting-style` gives the value the element
//! begins at, and the element's own `transition` longhands give which properties travel and how
//! long they take. Reading the motion out of those two is exact, costs nothing, and says the
//! same thing on every run — and it is the same reading the browser itself performs, so nothing
//! here is invented.
//!
//! Only the opening frame is emitted. A keyframes block that states `from` and stops animates
//! towards the element's own computed value, which is precisely the resting value the element
//! already carries and which the recreation therefore does not have to be told twice. Naming it
//! would also be impossible where it is the property's initial value, because a value equal to
//! the baseline is not recorded — it is what "unauthored" means.

use crate::model::{Animation, Node, Styles};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// One synthesized entry animation per property the author both starts and transitions.
pub(super) fn animations(
    before: &HashMap<String, Styles>,
    nodes: &[Node],
    recorded: &[Animation],
) -> Vec<Animation> {
    let mut built = Vec::new();
    for node in nodes {
        let Some(declared) = before.get(&node.path) else {
            continue;
        };
        for (property, from) in declared {
            let Some(timing) = transition_for(property, &node.style) else {
                continue;
            };
            if already_recorded(recorded, &node.path, property) {
                continue;
            }
            built.push(Animation {
                target: node.path.clone(),
                name: String::new(),
                keyframes: vec![opening(property, from, &timing.easing)],
                timing: json!({
                    "delay": timing.delay_ms,
                    "duration": timing.duration_ms,
                    "easing": timing.easing,
                    "fill": "backwards",
                    "iterations": 1,
                    "direction": "normal",
                    "playState": "running",
                }),
            });
        }
    }
    built
}

/// A property the capture already has motion for is left to the capture. The record it read is
/// a measurement of the running transition, which is the better authority for what happened;
/// this stage exists for the motion no reading was in time to see.
fn already_recorded(recorded: &[Animation], target: &str, property: &str) -> bool {
    recorded.iter().any(|animation| {
        animation.target == target
            && animation.keyframes.iter().any(|frame| {
                frame.as_object().is_some_and(|frame| {
                    frame
                        .keys()
                        .any(|key| super::animation_keyframes::kebab(key) == property)
                })
            })
    })
}

fn opening(property: &str, from: &str, easing: &str) -> Value {
    let mut frame = Map::new();
    frame.insert("offset".into(), json!(0));
    frame.insert("computedOffset".into(), json!(0));
    frame.insert("easing".into(), json!(easing));
    frame.insert(property.into(), json!(from));
    Value::Object(frame)
}

struct Timing {
    duration_ms: f64,
    delay_ms: f64,
    easing: String,
}

/// The transition the element declares for one property, or nothing if it declares none.
///
/// The longhands are parallel lists that repeat to the length of the property list, which is
/// how the shorthand expands, so the index a property sits at is the index every other value is
/// read at. `all` names every property at once and is read as the entry for whichever property
/// is being asked about.
fn transition_for(property: &str, style: &Styles) -> Option<Timing> {
    let properties = list(style.get("transition-property")?);
    let index = properties
        .iter()
        .position(|declared| declared == property || declared == "all")?;
    let durations = list(style.get("transition-duration")?);
    let duration_ms = milliseconds(cycled(&durations, index)?)?;
    if duration_ms <= 0.0 {
        return None;
    }
    let delays = style.get("transition-delay").map(|value| list(value));
    let easings = style
        .get("transition-timing-function")
        .map(|value| list(value));
    Some(Timing {
        duration_ms,
        delay_ms: delays
            .as_deref()
            .and_then(|delays| cycled(delays, index))
            .and_then(milliseconds)
            .unwrap_or(0.0),
        easing: easings
            .as_deref()
            .and_then(|easings| cycled(easings, index))
            .unwrap_or("linear")
            .to_string(),
    })
}

/// A comma-separated value list, split without breaking a function's own arguments — a
/// `cubic-bezier(0.4, 0, 0.2, 1)` is one entry, not four.
fn list(value: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for character in value.chars() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                entries.push(current.trim().to_string());
                current = String::new();
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    entries.push(current.trim().to_string());
    entries
}

fn cycled(values: &[String], index: usize) -> Option<&str> {
    if values.is_empty() {
        return None;
    }
    values.get(index % values.len()).map(String::as_str)
}

fn milliseconds(value: &str) -> Option<f64> {
    if let Some(number) = value.strip_suffix("ms") {
        return number.trim().parse().ok();
    }
    value
        .strip_suffix('s')
        .and_then(|number| number.trim().parse::<f64>().ok())
        .map(|seconds| seconds * 1000.0)
}

#[cfg(test)]
#[path = "starting_style_entry_tests.rs"]
mod tests;
