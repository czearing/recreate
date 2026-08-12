use super::before_change;
use super::css_values::append_class;
use crate::model::Animation;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub fn append(
    animations: &[Animation],
    authored: &BTreeSet<String>,
    starting: &before_change::BeforeChange,
    classes: &mut BTreeMap<String, String>,
    css: &mut String,
) {
    let animations = starting.seed(animations);
    let mut emitted_keyframes = BTreeSet::new();
    let mut targets: BTreeMap<&str, Vec<(String, String, &Animation)>> = BTreeMap::new();
    for animation in &animations {
        if animation.keyframes.len() < 2
            || sampled_layout_observation(animation)
            || authored.contains(&animation.name)
        {
            continue;
        }
        let digest = animation_digest(animation);
        let name = format!("recreate{}", &digest_of(&animation.keyframes)[..10]);
        if emitted_keyframes.insert(name.clone()) {
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
    authored: &BTreeSet<String>,
    starting: &before_change::BeforeChange,
    classes: &mut BTreeMap<String, String>,
    css: &mut String,
) {
    let startup = animations
        .iter()
        .filter(|animation| animation.target.starts_with("startup>"))
        .cloned()
        .collect::<Vec<_>>();
    append(&startup, authored, starting, classes, css);
}

/// Names a `@keyframes` block by the frames it is built from, because that is all
/// `animation_keyframes::append` reads. Two elements playing one movement at different
/// speeds share the block and differ only in the timing longhands that describe the
/// difference.
fn digest_of(value: &impl serde::Serialize) -> String {
    hex::encode(Sha256::digest(
        serde_json::to_vec(value).unwrap_or_default(),
    ))
}

/// Names the timed rule, which two elements share only when the movement *and* the way it is
/// played both agree.
fn animation_digest(animation: &Animation) -> String {
    digest_of(&(&animation.keyframes, &animation.timing))
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

#[cfg(test)]
#[path = "animation_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "animation_offset_tests.rs"]
mod offset_tests;
