use serde_json::{Value, json};

#[path = "rule_activation_fixture.rs"]
mod fixture;
use fixture::scene;

#[path = "rule_activation_base_tests.rs"]
mod base;
#[path = "rule_activation_condition_tests.rs"]
mod condition;
#[path = "capture_conditions_tests.rs"]
mod conditions;
#[path = "rule_activation_definition_tests.rs"]
mod definition;
#[path = "rule_activation_grouping_tests.rs"]
mod grouping;
#[path = "rule_activation_import_tests.rs"]
mod import;
#[path = "rule_activation_layer_tests.rs"]
mod layer;
#[path = "rule_activation_nested_rule_tests.rs"]
mod nested_rule;
#[path = "rule_activation_nesting_tests.rs"]
mod nesting;
#[path = "rule_activation_recovered_sheet_tests.rs"]
mod recovered_sheet;
#[path = "rule_activation_sheet_media_tests.rs"]
mod sheet_media;
#[path = "rule_activation_shorthand_tests.rs"]
mod shorthand;
#[path = "state_style_probe_tests.rs"]
mod state_style_probe;
#[path = "state_style_relation_tests.rs"]
mod state_style_relation;
#[path = "state_style_selector_tests.rs"]
mod state_style_selector;
#[path = "state_style_var_tests.rs"]
mod state_style_var;

const HARNESS: &str = concat!(
    include_str!("rule_activation_cssom.js"),
    "\n",
    include_str!("rule_activation_cssom_sheets.js"),
    "\n",
    include_str!("rule_activation_selectors.js"),
    "\n",
    include_str!("rule_activation_harness.js")
);

fn capture_source() -> String {
    format!(
        "{}\n{}\n{}\n{}",
        crate::selector_scan::SOURCE,
        crate::scoped_rules::SOURCE,
        crate::state_style_script::SOURCE
            .replace("__RULE_ACTIVATION__", super::SOURCE)
            .replace("__SHORTHAND_EXPANSION__", crate::capture_shorthands::SOURCE),
        crate::capture_conditions::source(),
    )
}

/// Runs the real capture walk over a scripted CSSOM and returns what it recorded.
fn walk(scene: Value) -> Value {
    walk_on(scene, HARNESS.to_string())
}

/// The same walk against a CSSOM whose interfaces the caller has restated. Chromium 151
/// answers `CSSStyleRule.prototype instanceof CSSGroupingRule` with `false`, but the CSSWG
/// has resolved to make a style rule a grouping rule, so the record must not depend on which
/// of the two an engine ships.
fn walk_on(scene: Value, harness: String) -> Value {
    let script = harness
        .replace("__SCENE__", &scene.to_string())
        .replace("__CAPTURE__", &capture_source());
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("walk.js");
    std::fs::write(&path, script).unwrap();
    let output = std::process::Command::new("node")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "walk failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// The double as an engine that has made a style rule a grouping rule.
fn grouping_style_rule_harness() -> String {
    let restated = HARNESS.replace(
        "class CSSStyleRule {}",
        "class CSSStyleRule extends CSSGroupingRule {}",
    );
    assert_ne!(
        restated, HARNESS,
        "the double no longer declares CSSStyleRule"
    );
    restated
}

fn style(selector: &str, name: &str, value: &str) -> Value {
    json!({ "selectorText": selector, "declarations": { name: value } })
}

/// The rule texts the walk recorded. A record also carries the base of the sheet that held
/// it, which `base` asserts separately.
fn recorded(result: &Value) -> Vec<String> {
    result["cssRules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| rule["text"].as_str().unwrap().to_string())
        .collect()
}

/// Generated output must not restate a rule. The same sheet reaches the walk twice, so a
/// walk that appends what it reads emits every authored rule twice into the stylesheet.
#[test]
fn a_sheet_read_twice_contributes_each_authored_rule_once() {
    let rules = recorded(&walk(scene()));
    let mut unique = rules.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        rules.len(),
        "duplicated authored rules: {rules:?}"
    );
    assert!(
        rules.iter().any(|rule| rule.starts_with(".panel")),
        "deduplication removed the rule entirely: {rules:?}"
    );
}

/// A definition rule is not a group. `@keyframes` exposes `cssRules`, so a walk that asks
/// "does it have children" rather than "is it a grouping rule" mistakes it for a wrapper
/// whose contents are recorded separately — and drops it, leaving every `animation-name`
/// that refers to it dangling. `@property` has no children and must survive alongside it.
#[test]
fn a_definition_rule_is_recorded_rather_than_treated_as_a_group() {
    let rules = recorded(&walk(scene()));
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with("@keyframes spin") && rule.contains("360deg")),
        "dropped an authored keyframes block: {rules:?}"
    );
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with("@property --angle")),
        "dropped an authored property registration: {rules:?}"
    );
    assert!(
        !rules
            .iter()
            .any(|rule| rule.trim_start().starts_with("from")),
        "descended into a keyframes block and recorded a keyframe as a rule: {rules:?}"
    );
}

/// A non-matching `@media` block is still authored responsive intent, so the block itself
/// stays; only its flattened, as-if-active copy must not.
#[test]
fn a_non_matching_media_block_survives_but_is_not_flattened() {
    let rules = recorded(&walk(scene()));
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with("@media (min-width: 900px)")),
        "dropped an authored media block: {rules:?}"
    );
    assert!(
        !rules
            .iter()
            .any(|rule| rule.starts_with(".panel") && rule.contains("color: red")),
        "flattened a media block that does not match: {rules:?}"
    );
}
