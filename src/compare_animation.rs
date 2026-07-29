use crate::model::{Animation, PageState};
use serde_json::{Map, Number, Value};
use std::collections::BTreeSet;

pub fn properties(state: &PageState, path: &str) -> BTreeSet<String> {
    let mut properties = state
        .animations
        .iter()
        .filter(|animation| animation.target == path)
        .flat_map(|animation| &animation.keyframes)
        .filter_map(serde_json::Value::as_object)
        .flat_map(|frame| frame.keys())
        .filter(|property| {
            !matches!(
                property.as_str(),
                "composite" | "computedOffset" | "easing" | "offset" | "x" | "y"
            )
        })
        .map(|property| kebab(property))
        .collect::<BTreeSet<_>>();
    for property in properties.clone() {
        if let Some(shorthand) = property
            .strip_suffix("-x")
            .or_else(|| property.strip_suffix("-y"))
        {
            properties.insert(shorthand.into());
        }
        if let Some(standard) = property
            .strip_prefix("-webkit-")
            .or_else(|| property.strip_prefix("webkit-"))
        {
            properties.insert(standard.into());
            if let Some(shorthand) = standard
                .strip_suffix("-x")
                .or_else(|| standard.strip_suffix("-y"))
            {
                properties.insert(shorthand.into());
            }
        }
    }
    properties
}

pub fn equivalent_at(expected: &PageState, actual: &PageState, target: &str) -> bool {
    let expected = signatures(expected, target);
    let actual = signatures(actual, target);
    !expected.is_empty() && expected == actual
}

pub fn equivalent_anywhere(expected: &PageState, actual: &PageState, target: &str) -> bool {
    let actual = signatures(actual, target);
    if actual.is_empty() {
        return false;
    }
    let mut expected = expected
        .animations
        .iter()
        .filter(|animation| !crate::generate::animations::sampled_layout_observation(animation))
        .map(signature)
        .collect::<Vec<_>>();
    actual.into_iter().all(|value| {
        expected
            .iter()
            .position(|candidate| candidate == &value)
            .map(|index| expected.remove(index))
            .is_some()
    })
}

pub fn phase_shifted_descendant(expected: &PageState, actual: &PageState, path: &str) -> bool {
    ancestors(path).any(|target| equivalent_at(expected, actual, target))
}

fn ancestors(path: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(path), |value| {
        value.rsplit_once('>').map(|(parent, _)| parent)
    })
}

fn signatures(state: &PageState, target: &str) -> Vec<String> {
    let mut values = state
        .animations
        .iter()
        .filter(|animation| {
            animation.target == target
                && !crate::generate::animations::sampled_layout_observation(animation)
        })
        .map(signature)
        .collect::<Vec<_>>();
    values.sort();
    values
}

fn signature(animation: &Animation) -> String {
    let keyframes = animation
        .keyframes
        .iter()
        .map(normalize_value)
        .collect::<Vec<_>>();
    let timing = animation
        .timing
        .as_object()
        .map(|values| {
            values
                .iter()
                .filter(|(key, _)| {
                    matches!(
                        key.as_str(),
                        "delay"
                            | "direction"
                            | "duration"
                            | "easing"
                            | "endDelay"
                            | "fill"
                            | "iterationStart"
                            | "iterations"
                            | "playbackRate"
                    )
                })
                .map(|(key, value)| (key.clone(), normalize_value(value)))
                .collect::<Map<_, _>>()
        })
        .unwrap_or_default();
    serde_json::to_string(&(keyframes, timing)).unwrap_or_default()
}

fn normalize_value(value: &Value) -> Value {
    match value {
        Value::Object(values) => Value::Object(
            values
                .iter()
                .filter(|(key, _)| !matches!(key.as_str(), "composite" | "computedOffset"))
                .map(|(key, value)| (key.clone(), normalize_value(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(normalize_value).collect()),
        Value::Number(value) => value
            .as_f64()
            .and_then(|value| Number::from_f64((value * 1000.0).round() / 1000.0))
            .map(Value::Number)
            .unwrap_or(Value::Null),
        _ => value.clone(),
    }
}

fn kebab(value: &str) -> String {
    let mut result = value.chars().fold(String::new(), |mut result, character| {
        if character.is_uppercase() {
            result.push('-');
            result.extend(character.to_lowercase());
        } else {
            result.push(character);
        }
        result
    });
    if result.starts_with("webkit-") {
        result.insert(0, '-');
    }
    result
}

#[cfg(test)]
#[path = "compare_animation_tests.rs"]
mod tests;
