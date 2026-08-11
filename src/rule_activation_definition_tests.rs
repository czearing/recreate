//! What the capture records for a definition at-rule that the author placed inside a
//! condition.
//!
//! Activation is measured by probing a rule's selector against the live document, so a rule
//! with no selector is never measured. Every entry starts out marked active, which for a
//! definition is the default it was given rather than anything observed — and recording it
//! on that basis publishes unconditionally what the page stated only under a condition.

use super::{recorded, style, walk};
use serde_json::{Value, json};

fn font_face(family: &str) -> Value {
    json!({ "prelude": "@font-face", "declarations": { "font-family": family } })
}

fn scene(prelude: &str, condition: &str, media: bool) -> Value {
    let sheet = json!([
        {
            "prelude": prelude,
            "conditionText": condition,
            "media": media,
            "rules": [font_face("Vorplish"), style(".panel", "color", "red")]
        },
        font_face("Quazitic"),
        style(".panel", "font-family", "Vorplish")
    ]);
    json!({
        "elements": [{ "path": "/main/div", "classes": ["panel"] }],
        "matching": { prelude: ["/main/div"] },
        "sheets": [sheet]
    })
}

fn media_scene() -> Value {
    scene("@media (min-width: 0px)", "(min-width: 0px)", true)
}

fn supports_scene() -> Value {
    scene("@supports (display: grid)", "(display: grid)", false)
}

/// The fabrication. A `@font-face` inside a live `@media` was recorded a second time on its
/// own, with the condition gone, so the generated page declared a font the source declared
/// only for one viewport range.
#[test]
fn records_no_unconditional_copy_of_a_definition_the_author_conditioned() {
    for (name, scene) in [("media", media_scene()), ("supports", supports_scene())] {
        let rules = recorded(&walk(scene));
        assert!(
            rules.iter().any(|rule| rule.contains("Vorplish")),
            "{name}: lost the conditioned definition entirely: {rules:?}"
        );
        assert!(
            !rules
                .iter()
                .any(|rule| rule.starts_with("@font-face") && rule.contains("Vorplish")),
            "{name}: recorded the definition stripped of the condition it was authored \
             under: {rules:?}"
        );
        assert!(
            rules
                .iter()
                .any(|rule| rule.contains("font-family: Vorplish;") && !rule.contains("@font-face")),
            "{name}: lost the reference, so the definition's fate is unattributable: \
             {rules:?}"
        );
    }
}

/// The survivor control. An unconditioned definition beside the conditioned one must still
/// be recorded bare, so the guard above cannot be satisfied by refusing definitions.
#[test]
fn still_records_a_definition_the_author_left_unconditional() {
    let rules = recorded(&walk(media_scene()));
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with("@font-face") && rule.contains("Quazitic")),
        "dropped an unconditional definition: {rules:?}"
    );
}

/// A definition inside a block recorded whole is already in the recorded text, so a bare
/// second copy is both a duplicate rule and an unconditional restatement of it. A style
/// rule beside it is measurable, and keeps being flattened past a condition found to hold.
#[test]
fn records_each_rule_inside_a_media_block_once() {
    let rules = recorded(&walk(media_scene()));
    let contained = rules
        .iter()
        .filter(|rule| rule.contains("Vorplish") && rule.contains("font-family"))
        .count();
    assert_eq!(
        contained, 2,
        "recorded the definition or its reference twice: {rules:?}"
    );
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with("@media (min-width: 0px)") && rule.contains("Vorplish")),
        "the surviving definition is not the conditioned one: {rules:?}"
    );
}
