//! The rule shapes the composed subject has to answer for, beside the compositions it
//! performs. A sub-module of `nested_rule` so it reuses that module's scene and helpers.

use super::super::{grouping_style_rule_harness, style, walk, walk_on};
use super::{element, scene, states};
use serde_json::json;

/// Whether a style rule is *also* a grouping rule is a CSSOM generation difference, not a
/// property of the page. Chromium 151 answers no; the CSSWG has resolved that it will answer
/// yes. A walk that asks "does this rule contain rules" to decide "is this rule a record"
/// therefore records every flat rule on one engine and none on the next, and the page cannot
/// tell you which you are on. Asking the narrower question first is what makes the two agree,
/// and this is the only place that agreement is observable.
#[test]
fn the_record_is_the_same_whether_or_not_a_style_rule_is_also_a_grouping_rule() {
    let mut scene = scene();
    scene["sheets"][0][0]["rules"] = json!([style("&:hover", "background-color", "#b00020")]);

    let shipped = states(&walk(scene.clone()));
    let restated = states(&walk_on(scene, grouping_style_rule_harness()));

    assert!(
        shipped.contains(&"/page/tab|:hover|background-color: #b00020;".into()),
        "the nested state rule was lost on the engine that ships today: {shipped:#?}"
    );
    assert_eq!(
        restated, shipped,
        "the record changed with the CSSOM generation rather than with the page"
    );
}

/// The rule shape whose subject is nowhere in its own text. A run of declarations sitting
/// inside a nested group rule is wrapped in a nested declarations rule, which carries a block
/// and no selector, and matches exactly what its parent matches. A walk that asks a rule for
/// its `selectorText` before reading it skips this shape entirely.
#[test]
fn a_block_with_no_selector_of_its_own_is_read_against_its_parent() {
    let scene = json!({
        "elements": [element("/page/btn", json!(["btn"]))],
        "matching": { "@media (prefers-color-scheme: dark)": ["/page/btn"] },
        "sheets": [[{
            "selectorText": ".btn",
            "declarations": { "color": "#101010" },
            "rules": [{
                "selectorText": "&:hover",
                "declarations": { "color": "#202020" },
                "rules": [{
                    "prelude": "@media (prefers-color-scheme: dark)",
                    "conditionText": "(prefers-color-scheme: dark)",
                    "rules": [{ "nestedDeclarations": { "background-color": "#37474f" } }]
                }]
            }]
        }]]
    });
    let recorded = states(&walk(scene));

    assert!(
        recorded.contains(&"/page/btn|:hover|background-color: #37474f;".into()),
        "a block with no selector of its own was never read: {recorded:#?}"
    );
}

/// The masking control. `@keyframes` also exposes children, and its children carry blocks,
/// but they are keyframe selectors rather than rules the cascade resolves. A walk that
/// descends by "does it have children" records `from` and `to` as authored selectors.
#[test]
fn keyframe_steps_are_still_not_read_as_rules() {
    let mut scene = scene();
    scene["sheets"][0] = json!([{
        "prelude": "@keyframes pulse",
        "keyframes": true,
        "rules": [
            style("from", "background-color", "#ff6f00"),
            style("to", "background-color", "#ff6f00")
        ]
    }]);
    let recorded = states(&walk(scene));

    assert!(
        recorded.is_empty(),
        "descended into a keyframes block and read its steps as rules: {recorded:#?}"
    );
}

/// Generated output must not restate a rule. A style rule serialises its children inside its
/// own block, so recording a child again publishes the same declarations twice — which is
/// exactly what a walk that starts descending would do if it also kept recording.
#[test]
fn a_nested_rule_is_not_recorded_again_beside_the_parent_that_already_holds_it() {
    let mut scene = scene();
    scene["sheets"][0][0]["rules"] = json!([style("&:hover", "background-color", "#b00020")]);
    let recorded = super::super::recorded(&walk(scene));
    let carrying = recorded
        .iter()
        .filter(|text| text.contains("#b00020"))
        .count();

    assert_eq!(
        carrying, 1,
        "the nested rule's declarations were published more than once: {recorded:#?}"
    );
}
