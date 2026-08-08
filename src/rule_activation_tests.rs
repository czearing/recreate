use serde_json::{Value, json};

#[path = "rule_activation_layer_tests.rs"]
mod layer;

const HARNESS: &str = include_str!("rule_activation_harness.js");

fn capture_source() -> String {
    crate::state_style_script::SOURCE.replace("__RULE_ACTIVATION__", super::SOURCE)
}

/// Runs the real capture walk over a scripted CSSOM and returns what it recorded.
fn walk(scene: Value) -> Value {
    let script = HARNESS
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

fn style(selector: &str, name: &str, value: &str) -> Value {
    json!({ "selectorText": selector, "declarations": { name: value } })
}

fn recorded(result: &Value) -> Vec<String> {
    result["cssRules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| rule.as_str().unwrap().to_string())
        .collect()
}

/// The scene: a 300px container, a false `@supports` condition, a `@container` condition
/// that a 300px container cannot satisfy, and — so that dropping every grouped rule cannot
/// pass — conditions that do hold. The sheet is supplied twice, which is what the capture
/// does when it cannot tell which sheets the page failed to read.
fn scene() -> Value {
    let sheet = json!([
        style(".panel", "padding", "24px"),
        {
            "prelude": "@container panelwrap (min-width: 900px)",
            "conditionText": "panelwrap (min-width: 900px)",
            "rules": [style(".panel", "width", "100%")]
        },
        {
            "prelude": "@supports (color: nonexistent-color-function(1))",
            "conditionText": "(color: nonexistent-color-function(1))",
            "rules": [style(".panel", "max-width", "50%")]
        },
        {
            "prelude": "@supports (display: grid)",
            "conditionText": "(display: grid)",
            "rules": [style(".grid", "display", "grid")]
        },
        {
            "prelude": "@media (min-width: 900px)",
            "conditionText": "(min-width: 900px)",
            "media": true,
            "rules": [style(".panel", "color", "red")]
        },
        {
            "prelude": "@media (min-width: 0px)",
            "conditionText": "(min-width: 0px)",
            "media": true,
            "rules": [{
                "prelude": "@supports (display: grid)",
                "conditionText": "(display: grid)",
                "rules": [style(".wide", "gap", "8px")]
            }]
        },
        {
            "prelude": "@keyframes spin",
            "keyframes": true,
            "rules": [style("from", "rotate", "0deg"), style("to", "rotate", "360deg")]
        },
        { "prelude": "@property --angle", "declarations": { "syntax": "'<angle>'" } }
    ]);
    json!({
        "elements": [
            { "path": "/main/div", "classes": ["wrap"] },
            { "path": "/main/div/div", "classes": ["panel"] },
            { "path": "/main/div/span", "classes": ["grid"] },
            { "path": "/main/div/p", "classes": ["wide"] }
        ],
        "matching": {
            "@supports (display: grid)": ["/main/div", "/main/div/div", "/main/div/span", "/main/div/p"],
            "@media (min-width: 0px)": ["/main/div", "/main/div/div", "/main/div/span", "/main/div/p"]
        },
        "sheets": [sheet.clone(), sheet]
    })
}

/// A `@container` block the page's own layout can never satisfy declares nothing, so its
/// declarations must not be recorded as rules the author wrote.
#[test]
fn an_unsatisfiable_container_block_contributes_no_authored_rule() {
    let rules = recorded(&walk(scene()));
    assert!(
        !rules.iter().any(|rule| rule.contains("width: 100%")),
        "recorded a dead @container declaration: {rules:?}"
    );
}

/// `@supports` keeps its nested rules in the CSSOM when the condition is false, so a walk
/// that reads them without evaluating the condition records styles no browser applies.
#[test]
fn a_false_supports_block_contributes_no_authored_rule() {
    let rules = recorded(&walk(scene()));
    assert!(
        !rules.iter().any(|rule| rule.contains("max-width: 50%")),
        "recorded a dead @supports declaration: {rules:?}"
    );
}

/// The inverse of the two tests above. Discarding every grouped rule would satisfy them
/// while destroying the styles that do apply, so both directions are asserted.
#[test]
fn a_satisfied_condition_still_contributes_its_authored_rule() {
    let rules = recorded(&walk(scene()));
    assert!(
        rules.iter().any(|rule| rule.starts_with(".grid")),
        "lost a live @supports declaration: {rules:?}"
    );
    assert!(
        rules.iter().any(|rule| rule.starts_with(".wide")),
        "lost a declaration nested in two satisfied conditions: {rules:?}"
    );
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with(".panel") && rule.contains("padding")),
        "lost an unconditional declaration: {rules:?}"
    );
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
