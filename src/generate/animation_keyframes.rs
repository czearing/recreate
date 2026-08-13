use crate::model::Animation;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// The animation names the emitted stylesheet defines.
///
/// An animation the author declared in CSS is already reproduced in full by those
/// keyframes plus the element's baked computed style, which carries the name and every
/// timing longhand. Rebuilding it from sampled frames writes a second `animation-name`
/// over the first, so the authored definition would sit in the output unused — and a
/// paused or slow animation samples to frames that describe no change at all.
///
/// It reads the stylesheet that was emitted rather than the rules it was built from,
/// because the question is what this pipeline published, not what the page contained. Those
/// two answers differ whenever emission drops something, and re-deriving the second from
/// the captured rules is a second implementation of the emission decision that nothing
/// keeps in step. It already fell out of step once: `css::retain` learned to descend into
/// grouping at-rules, this stage kept filtering the top level, and a `@keyframes` nested in
/// a condition was emitted correctly and then reported as undefined — so the sampler
/// rebuilt it under a generated name and overrode the author's, publishing unconditionally
/// what the author had written under a condition.
///
/// Reading the output makes that disagreement unrepresentable rather than merely tested
/// for, and any later change to what gets emitted is reflected here with no second opinion
/// to update.
pub fn authored_names(css: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for rule in super::css_rule_split::top_level(css).iter() {
        // `retain` owns the descent; this closure owns only what counts as a definition.
        let _ = super::css::retain(rule, &mut |member| match keyframes_name(member) {
            Some(name) => names.insert(name),
            None => false,
        });
    }
    names
}

/// The name a `@keyframes` block defines, or `None` for any other rule.
///
/// The vendor-prefixed spelling defines the same name, so it is read through rather than
/// listed. A grouping rule is rejected by having no `keyframes` keyword, which matters
/// because splitting `@media (min-width: 1px)` on whitespace yields a token that reads like
/// an identifier and would otherwise be recorded as a name no element can reference.
fn keyframes_name(rule: &str) -> Option<String> {
    let (_, rule) = super::css_layers::peel(rule);
    let prelude = rule.strip_prefix('@')?.split('{').next()?;
    let (keyword, name) = prelude.trim_end().split_once(char::is_whitespace)?;
    keyword
        .trim_start_matches("-webkit-")
        .eq_ignore_ascii_case("keyframes")
        .then(|| name.trim().trim_matches(['"', '\'']).to_string())
        .filter(|name| !name.is_empty())
}

#[cfg(test)]
#[path = "animation_authored_names_tests.rs"]
mod authored_names_tests;
#[cfg(test)]
#[path = "animation_emitted_source_tests.rs"]
mod emitted_source_tests;
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

pub(super) fn kebab(value: &str) -> String {
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
