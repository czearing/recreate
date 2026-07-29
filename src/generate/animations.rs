use crate::model::Animation;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub fn append(animations: &[Animation], classes: &mut BTreeMap<String, String>, css: &mut String) {
    let mut emitted_keyframes = BTreeSet::new();
    let mut targets: BTreeMap<&str, Vec<(String, String, &Animation)>> = BTreeMap::new();
    for animation in animations {
        if animation.keyframes.len() < 2 || sampled_layout_observation(animation) {
            continue;
        }
        let digest = animation_digest(animation);
        let name = format!("recreate{}", &digest[..10]);
        if emitted_keyframes.insert(digest.clone()) {
            super::animation_keyframes::append(animation, &name, css);
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
            css.push_str(&format!(".{class}{{animation:none!important;}}"));
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

pub(crate) fn sampled_layout_observation(animation: &Animation) -> bool {
    let has_geometry = animation.keyframes.iter().any(|frame| {
        frame.as_object().is_some_and(|values| {
            values
                .keys()
                .any(|key| matches!(key.as_str(), "x" | "y" | "width" | "height"))
        })
    });
    let has_effect_metadata = animation.keyframes.iter().any(|frame| {
        frame.as_object().is_some_and(|values| {
            values
                .keys()
                .any(|key| matches!(key.as_str(), "computedOffset" | "composite" | "easing"))
        })
    });
    let timing = animation.timing.as_object();
    has_geometry
        && !has_effect_metadata
        && timing.is_some_and(|values| {
            !values.contains_key("direction")
                && !values.contains_key("fill")
                && !values.contains_key("playState")
                && !values.contains_key("playbackRate")
        })
}

fn append_class(value: &mut String, class: &str) {
    if !value.split_whitespace().any(|item| item == class) {
        value.push(' ');
        value.push_str(class);
    }
}

#[cfg(test)]
#[path = "animation_tests.rs"]
mod tests;
