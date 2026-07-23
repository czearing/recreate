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
mod tests {
    use super::*;
    use crate::model::{PageState, Viewport};
    use serde_json::json;

    fn state(target: &str, duration: f64) -> PageState {
        PageState {
            url: String::new(),
            title: String::new(),
            viewport: Viewport::default(),
            nodes: Vec::new(),
            dom: Default::default(),
            capture_blockers: Vec::new(),
            startup_nodes: Vec::new(),
            startup_delay_ms: 0,
            startup_duration_ms: 0,
            animations: vec![Animation {
                target: target.into(),
                keyframes: vec![
                    json!({"offset":0.0,"opacity":"0","easing":"ease-in-out"}),
                    json!({"offset":1.0,"opacity":"1","easing":"ease-in-out"}),
                ],
                timing: json!({"duration":duration,"iterations":"infinite","easing":"linear"}),
            }],
            state_styles: Vec::new(),
            attribute_sequences: Vec::new(),
            css_rules: Vec::new(),
            asset_urls: Vec::new(),
            asset_data: Default::default(),
        }
    }

    #[test]
    fn recognizes_equivalent_browser_animation_observations() {
        let expected = state("html>body>div:nth-of-type(1)", 16000.0);
        let mut actual = expected.clone();
        actual.animations[0].keyframes[0]["computedOffset"] = json!(0.0);

        assert!(equivalent_at(
            &expected,
            &actual,
            "html>body>div:nth-of-type(1)"
        ));
        assert!(phase_shifted_descendant(
            &expected,
            &actual,
            "html>body>div:nth-of-type(1)>span:nth-of-type(1)"
        ));
    }

    #[test]
    fn rejects_different_animation_timing() {
        let expected = state("html>body", 16000.0);
        let actual = state("html>body", 9000.0);
        assert!(!equivalent_at(&expected, &actual, "html>body"));
    }

    #[test]
    fn recognizes_equivalent_animation_on_a_rebased_target() {
        let expected = state("html>body>main", 16000.0);
        let actual = state("html>body>section", 16000.0);
        assert!(equivalent_anywhere(&expected, &actual, "html>body>section"));
    }

    #[test]
    fn exposes_browser_webkit_animation_properties_as_css_names() {
        let mut expected = state("html>body>main", 16000.0);
        expected.animations[0].keyframes = vec![
            json!({"webkitMaskPositionX":"0%"}),
            json!({"webkitMaskPositionX":"100%"}),
        ];

        let properties = properties(&expected, "html>body>main");
        assert!(properties.contains("-webkit-mask-position-x"));
        assert!(properties.contains("-webkit-mask-position"));
        assert!(properties.contains("mask-position-x"));
        assert!(properties.contains("mask-position"));
    }
}
