use crate::model::Animation;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub fn append(animations: &[Animation], classes: &mut BTreeMap<String, String>, css: &mut String) {
    let mut emitted_keyframes = BTreeSet::new();
    let mut targets: BTreeMap<&str, Vec<(String, String, &Animation)>> = BTreeMap::new();
    for animation in animations {
        if animation.keyframes.len() < 2 {
            continue;
        }
        let digest = animation_digest(animation);
        let name = format!("recreate{}", &digest[..10]);
        if emitted_keyframes.insert(digest.clone()) {
            append_keyframes(animation, &name, css);
        }
        targets
            .entry(&animation.target)
            .or_default()
            .push((digest, name, animation));
    }
    let mut emitted_classes = BTreeSet::new();
    let mut reduced_classes = Vec::new();
    for (target, rules) in targets {
        let signature = rules
            .iter()
            .map(|(digest, _, _)| digest.as_str())
            .collect::<Vec<_>>()
            .join("|");
        let digest = hex::encode(Sha256::digest(signature));
        let class = format!("a{}", &digest[..10]);
        if emitted_classes.insert(digest) {
            let names: Vec<String> = rules.iter().map(|(_, name, _)| name.clone()).collect();
            let animations: Vec<&Animation> =
                rules.iter().map(|(_, _, animation)| *animation).collect();
            css.push_str(&format!(
                ".{class}{{{}}}\n",
                super::animation_timing::declarations(&animations, &names)
            ));
            reduced_classes.push(class.clone());
        }
        classes
            .entry(target.into())
            .and_modify(|value| append_class(value, &class));
    }
    if !reduced_classes.is_empty() {
        css.push_str("@media (prefers-reduced-motion: reduce){");
        for class in reduced_classes {
            css.push_str(&format!(
                ".{class}{{animation:none!important;transition:none!important;}}"
            ));
        }
        css.push_str("}\n");
    }
}

pub fn append_startup(
    animations: &[Animation],
    classes: &mut BTreeMap<String, String>,
    css: &mut String,
) {
    let startup = animations
        .iter()
        .filter(|animation| animation.target.starts_with("startup>"))
        .cloned()
        .collect::<Vec<_>>();
    append(&startup, classes, css);
}

fn animation_digest(animation: &Animation) -> String {
    let signature =
        serde_json::to_vec(&(&animation.keyframes, &animation.timing)).unwrap_or_default();
    hex::encode(Sha256::digest(signature))
}

fn append_keyframes(animation: &Animation, name: &str, css: &mut String) {
    let final_position = position(animation.keyframes.last());
    css.push_str(&format!("@keyframes {name}{{"));
    let mut frames: BTreeMap<i32, Map<String, Value>> = BTreeMap::new();
    for (index, frame) in animation.keyframes.iter().enumerate() {
        let offset = frame["offset"]
            .as_f64()
            .unwrap_or(index as f64 / (animation.keyframes.len() - 1) as f64);
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

fn append_class(value: &mut String, class: &str) {
    if !value.split_whitespace().any(|item| item == class) {
        value.push(' ');
        value.push_str(class);
    }
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

fn declarations(values: Option<&Map<String, Value>>, final_position: (f64, f64)) -> String {
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
    value.chars().fold(String::new(), |mut result, character| {
        if character.is_uppercase() {
            result.push('-');
            result.extend(character.to_lowercase());
        } else {
            result.push(character);
        }
        result
    })
}

#[cfg(test)]
#[path = "animation_tests.rs"]
mod tests;
