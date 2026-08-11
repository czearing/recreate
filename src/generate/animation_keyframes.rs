use crate::model::Animation;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// The names of the `@keyframes` blocks the emitted authored CSS defines.
///
/// An animation the author declared in CSS is already reproduced in full by those
/// keyframes plus the element's baked computed style, which carries the name and every
/// timing longhand. Rebuilding it from sampled frames writes a second `animation-name`
/// over the first, so the authored definition would sit in the output unused — and a
/// paused or slow animation samples to frames that describe no change at all.
pub fn authored_names(rules: &[String]) -> BTreeSet<String> {
    rules
        .iter()
        .filter(|rule| super::css::global_rule(rule))
        .filter_map(|rule| {
            let (_, rule) = super::css_layers::peel(rule);
            let prelude = rule.strip_prefix('@')?.split('{').next()?;
            let (keyword, name) = prelude.trim_end().split_once(char::is_whitespace)?;
            keyword
                .trim_start_matches("-webkit-")
                .eq_ignore_ascii_case("keyframes")
                .then(|| name.trim().trim_matches(['"', '\'']).to_string())
        })
        .collect()
}

pub fn append(animation: &Animation, name: &str, css: &mut String) {
    let final_position = position(animation.keyframes.last());
    css.push_str(&format!("@keyframes {name}{{"));
    let mut frames: BTreeMap<i32, Map<String, Value>> = BTreeMap::new();
    for (index, frame) in animation.keyframes.iter().enumerate() {
        let offset = recorded_offset(frame, index, animation.keyframes.len());
        if let Some(values) = frame.as_object() {
            frames
                .entry((offset * 100.0).round() as i32)
                .or_default()
                .extend(values.clone());
        }
    }
    for (offset, frame) in frames {
        css.push_str(&format!(
            "{offset}%{{{}}}",
            declarations(Some(&frame), final_position)
        ));
    }
    css.push_str("}\n");
}

/// The place on the timeline the browser recorded for this frame.
///
/// `getKeyframes()` reports `offset` only where the author stated one and `computedOffset`
/// for every frame, so the recorded place is the computed one. Spacing by index is a last
/// resort for records carrying neither: it spreads frames evenly across the whole list,
/// which is what the browser computes only when no interior frame is anchored. Anchor one
/// and the two disagree, and since places are rounded to a whole percentage and merged by
/// key, a guess landing on a recorded place overwrites that frame rather than mistiming it.
fn recorded_offset(frame: &Value, index: usize, count: usize) -> f64 {
    frame["computedOffset"]
        .as_f64()
        .or_else(|| frame["offset"].as_f64())
        .unwrap_or(index as f64 / count.saturating_sub(1).max(1) as f64)
}

fn position(frame: Option<&Value>) -> (f64, f64) {
    frame
        .and_then(Value::as_object)
        .map(|values| {
            (
                values.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                values.get("y").and_then(Value::as_f64).unwrap_or(0.0),
            )
        })
        .unwrap_or_default()
}

pub(super) fn declarations(
    values: Option<&Map<String, Value>>,
    final_position: (f64, f64),
) -> String {
    let Some(values) = values else {
        return String::new();
    };
    let mut output = String::new();
    let x = values.get("x").and_then(Value::as_f64);
    let y = values.get("y").and_then(Value::as_f64);
    if x.is_some() || y.is_some() {
        output.push_str(&format!(
            "translate:{}px {}px;",
            x.unwrap_or(final_position.0) - final_position.0,
            y.unwrap_or(final_position.1) - final_position.1
        ));
    }
    for (key, value) in values {
        if key == "easing" {
            if let Some(value) = value.as_str() {
                output.push_str(&format!("animation-timing-function:{value};"));
            }
            continue;
        }
        if ["offset", "composite", "computedOffset", "x", "y"].contains(&key.as_str()) {
            continue;
        }
        if let Some(value) = css_value(key, value) {
            output.push_str(&format!("{}:{value};", kebab(key)));
        }
    }
    output
}

fn css_value(key: &str, value: &Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.into());
    }
    value.as_f64().map(|value| {
        if matches!(key, "width" | "height" | "left" | "top") {
            format!("{value}px")
        } else {
            value.to_string()
        }
    })
}

fn kebab(value: &str) -> String {
    let result = value.chars().fold(String::new(), |mut result, character| {
        if character.is_uppercase() {
            result.push('-');
            result.extend(character.to_lowercase());
        } else {
            result.push(character);
        }
        result
    });
    result
        .strip_prefix("webkit-")
        .map(|value| format!("-webkit-{value}"))
        .unwrap_or(result)
}
