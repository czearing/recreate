//! What a conditional group records when another conditional group encloses it.
//!
//! Wrapping is the whole variable here. The wrapped sheets differ from the unwrapped
//! control by exactly one prelude, so anything that changes between them is caused by the
//! wrapping and by nothing else.

use super::{recorded, style, walk};
use serde_json::{Value, json};

/// `.subject`'s media rule sits inside a feature query that holds, `.hidden`'s inside one
/// that cannot, `.reversed` nests the two the other way, and `.control`'s sits inside
/// nothing. The media condition is false at this width in all four, so a value's presence
/// proves the authored text was carried rather than re-measured.
fn scene() -> Value {
    let sheet = json!([
        {
            "prelude": "@media (min-width: 900px)",
            "conditionText": "(min-width: 900px)",
            "rules": [style(".control", "letter-spacing", "11px")]
        },
        {
            "prelude": "@supports (display: grid)",
            "conditionText": "(display: grid)",
            "rules": [{
                "prelude": "@media (min-width: 900px)",
                "conditionText": "(min-width: 900px)",
                "rules": [style(".subject", "letter-spacing", "13px")]
            }]
        },
        {
            "prelude": "@supports (color: nonexistent-color-function(1))",
            "conditionText": "(color: nonexistent-color-function(1))",
            "rules": [{
                "prelude": "@media (min-width: 900px)",
                "conditionText": "(min-width: 900px)",
                "rules": [style(".hidden", "letter-spacing", "17px")]
            }]
        },
        {
            "prelude": "@media (min-width: 900px)",
            "conditionText": "(min-width: 900px)",
            "rules": [{
                "prelude": "@supports (display: grid)",
                "conditionText": "(display: grid)",
                "rules": [style(".reversed", "letter-spacing", "19px")]
            }]
        }
    ]);
    json!({
        "elements": [
            { "path": "/main/p:nth-of-type(1)", "classes": ["control"] },
            { "path": "/main/p:nth-of-type(2)", "classes": ["subject"] },
            { "path": "/main/p:nth-of-type(3)", "classes": ["hidden"] },
            { "path": "/main/p:nth-of-type(4)", "classes": ["reversed"] }
        ],
        "matching": {
            "@supports (display: grid)": [
                "/main/p:nth-of-type(1)", "/main/p:nth-of-type(2)",
                "/main/p:nth-of-type(3)", "/main/p:nth-of-type(4)"
            ]
        },
        "sheets": [sheet]
    })
}

fn carrying(rules: &[String], value: &str) -> Vec<String> {
    rules
        .iter()
        .filter(|rule| rule.contains(value))
        .cloned()
        .collect()
}

/// The invariant the gate/carrier split exists to state: what survives must not depend on
/// what happens to wrap it. A `@media` rule authored inside a feature query that holds must
/// be recorded as the identical top-level media rule — carried, and carried *without* the
/// feature query, because re-emitting a gate would make the recreation re-ask the viewing
/// engine a question the capturing engine already answered.
#[test]
fn wrapping_a_media_rule_in_a_condition_that_holds_changes_nothing_about_what_survives() {
    let rules = recorded(&walk(scene()));
    let control = carrying(&rules, "letter-spacing: 11px");
    let subject = carrying(&rules, "letter-spacing: 13px");

    assert_eq!(control.len(), 1, "lost the unwrapped control: {rules:?}");
    assert_eq!(
        subject.len(),
        1,
        "a media rule under a satisfied feature query was not carried exactly once: {rules:?}"
    );
    assert_eq!(
        control[0].replace(".control", ".x").replace("11px", "0"),
        subject[0].replace(".subject", ".x").replace("13px", "0"),
        "wrapping changed the recorded text"
    );
    assert!(
        !rules.iter().any(|rule| rule.contains("@supports")),
        "re-emitted a gate the capturing engine already answered: {rules:?}"
    );
}

/// The reverse nesting, which CSS Conditional Rules 3 permits just as freely. The media
/// condition still has to be carried and the feature query still has to be dropped, so a
/// walk that understands only one order closes half the defect.
#[test]
fn a_condition_that_holds_inside_a_media_rule_is_dropped_and_the_media_rule_kept() {
    let rules = recorded(&walk(scene()));
    let reversed = carrying(&rules, "letter-spacing: 19px");

    assert_eq!(reversed.len(), 1, "lost the reverse nesting: {rules:?}");
    assert!(
        reversed[0].starts_with("@media (min-width: 900px)") && !reversed[0].contains("@supports"),
        "did not reduce the reverse nesting to a bare media rule: {reversed:?}"
    );
}

/// The opposite direction of the same rule, and the reason a media rule may not simply be
/// lifted out of whatever encloses it. A feature query that is false declares nothing, so
/// the media rule inside it declares nothing either, however well-formed its own condition
/// is. Only a measured gate tells the two apart.
#[test]
fn a_media_rule_wrapped_in_a_false_feature_query_contributes_nothing() {
    let rules = recorded(&walk(scene()));
    assert!(
        !rules
            .iter()
            .any(|rule| rule.contains("letter-spacing: 17px")),
        "recorded a media rule hidden behind a false feature query: {rules:?}"
    );
}
