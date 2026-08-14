//! A condition that lives on the sheet rather than in it.
//!
//! `<style media>`, `<link media>` and the query trailing an `@import` prelude all condition
//! every rule in a sheet without ever appearing in `cssRules`. A walk that reads only rule
//! text therefore records those rules as unconditional, and — because the sentinel probe
//! runs only for entries carrying a gate — as active by a default nobody decided. The
//! import route is exercised in `rule_activation_script::tests::import`, which has to reach
//! the sheet before it can carry anything.

use super::{recorded as recorded_rules, style, walk};
use serde_json::{Value, json};

fn scene(sheets: Value, extra: Value) -> Value {
    let mut base = json!({
        "elements": [
            { "path": "/attr", "classes": ["attr"] },
            { "path": "/rule", "classes": ["rule"] },
            { "path": "/plain", "classes": ["plain"] }
        ],
        "matching": {},
        "sheets": sheets
    });
    for (key, value) in extra.as_object().unwrap() {
        base[key] = value.clone();
    }
    base
}

fn rules(sheets: Value) -> Vec<String> {
    recorded_rules(&walk(scene(sheets, json!({}))))
}

fn holding(rules: &[String], needle: &str) -> Vec<String> {
    rules
        .iter()
        .filter(|rule| rule.contains(needle))
        .cloned()
        .collect()
}

fn only(rules: &[String], needle: &str) -> String {
    let found = holding(rules, needle);
    assert_eq!(
        found.len(),
        1,
        "expected exactly one rule for {needle}: {rules:?}"
    );
    found.into_iter().next().unwrap()
}

/// The subject. `media="print"` was false for the whole capture, so the page never applied
/// this declaration — yet it is recorded, and recorded as though the author had written it
/// unconditionally. That is fabrication rather than loss: an authored id selector outranks
/// the generated single-class bake, so the recreation paints a colour the source never did.
#[test]
fn a_condition_on_the_sheet_travels_with_the_rules_it_guards() {
    let recorded = rules(json!([
        { "media": "print", "rules": [style(".attr", "color", "green")] }
    ]));
    assert_eq!(
        only(&recorded, "green"),
        "@media print{.attr { color: green; }}",
        "a sheet-level condition was discarded, publishing a rule the page never applied"
    );
}

/// The twin relation, which names no literal and so survives any repair: the same condition
/// must produce the same record whichever route the author spelled it through.
#[test]
fn the_attribute_route_records_what_the_rule_route_records() {
    let recorded = rules(json!([
        { "media": "print", "rules": [style(".attr", "color", "green")] },
        [{
            "prelude": "@media print",
            "conditionText": "print",
            "rules": [style(".rule", "color", "green")]
        }]
    ]));
    let attr = only(&recorded, ".attr").replace(".attr", "X");
    let rule = only(&recorded, ".rule").replace(".rule", "X");
    assert_eq!(attr, rule, "the two routes disagree: {recorded:?}");
}

/// The other direction, and the reason the cheap repair is forbidden. This condition held
/// for the whole capture, so dropping the sheet's rules would delete a declaration the page
/// genuinely applied; keeping them unconditioned would republish a guarded rule as absolute.
/// Only carrying the condition is correct, and one seed produces both outcomes.
#[test]
fn a_condition_that_held_at_capture_is_kept_and_still_wrapped() {
    let recorded = rules(json!([
        { "media": "(min-width: 300px)", "rules": [style(".attr", "color", "magenta")] }
    ]));
    assert_eq!(
        only(&recorded, "magenta"),
        "@media (min-width: 300px){.attr { color: magenta; }}"
    );
}

/// The inverse guard. Every existing fixture is a sheet with no condition, so this is the
/// assertion that the repair is a no-op for them.
#[test]
fn a_sheet_with_no_condition_is_recorded_exactly_as_before() {
    let recorded = rules(json!([[style(".plain", "color", "red")]]));
    assert_eq!(recorded, vec![".plain { color: red; }"]);
}

/// A sheet condition and a rule condition nest, exactly as two `@media` blocks would. The
/// order matters: the sheet encloses the rule, because the sheet is the outer scope.
#[test]
fn a_sheet_condition_encloses_a_media_rule_written_inside_it() {
    let recorded = rules(json!([{
        "media": "print",
        "rules": [{
            "prelude": "@media (min-width: 900px)",
            "conditionText": "(min-width: 900px)",
            "rules": [style(".attr", "color", "green")]
        }]
    }]));
    assert_eq!(
        only(&recorded, "green"),
        "@media print{@media (min-width: 900px){.attr { color: green; }}}"
    );
}

/// A `@supports` written inside a conditioned sheet is still the agent's answer to give, so
/// it is evaluated and dropped while the sheet's own condition survives. Guards against a
/// repair that seeds the gate stack instead of the carrier stack, which would make the
/// sheet's condition decide activation and delete the rule outright.
#[test]
fn a_sheet_condition_does_not_become_a_gate() {
    let recorded = recorded_rules(&walk(scene(
        json!([{
            "media": "print",
            "rules": [{
                "prelude": "@supports (display: grid)",
                "conditionText": "(display: grid)",
                "rules": [style(".attr", "color", "green")]
            }]
        }]),
        json!({ "matching": { "@supports (display: grid)": ["/attr"] } }),
    )));
    assert_eq!(
        only(&recorded, "green"),
        "@media print{.attr { color: green; }}",
        "the sheet's condition must be carried, and @supports still evaluated away"
    );
}

/// A state rule is recorded whatever its condition, because the state it describes is
/// entered later. The condition it was authored under has to travel with it, or a hover
/// colour spelled for print is replayed on screen. This is the half of the seed the
/// recorded rule text cannot show.
#[test]
fn a_state_rule_carries_the_condition_of_the_sheet_that_holds_it() {
    let result = walk(scene(
        json!([{ "media": "print", "rules": [style(".attr:hover", "color", "green")] }]),
        json!({}),
    ));
    let states = result["stateStyles"].as_array().unwrap();
    assert_eq!(states.len(), 1, "{states:?}");
    assert_eq!(states[0]["media"], "print");
}

/// Two conditions both hold or the rule does not apply, so a sheet condition and a rule
/// condition compose rather than replace. Spelled as the conjunction the walk already
/// builds for nested media rules, so one rule serves both nestings.
#[test]
fn a_sheet_condition_composes_with_a_media_rule_inside_it() {
    let result = walk(scene(
        json!([{
            "media": "print",
            "rules": [{
                "prelude": "@media (min-width: 900px)",
                "conditionText": "(min-width: 900px)",
                "rules": [style(".attr:hover", "color", "green")]
            }]
        }]),
        json!({}),
    ));
    let states = result["stateStyles"].as_array().unwrap();
    assert_eq!(states.len(), 1, "{states:?}");
    assert_eq!(states[0]["media"], "(print) and ((min-width: 900px))");
}
