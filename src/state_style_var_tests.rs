//! How a state rule's `var()` references are substituted on the way into the record.
//!
//! A state declaration is captured as text, so the capture has to perform the substitution the
//! browser would have performed on hover. `var()` takes `var( <name> [, <declaration-value> ]? )`
//! and a fallback is arbitrary CSS: it may carry commas, quoted strings and nested functions.
//! Finding where such a function ends is a balanced-delimiter problem, so these tests drive the
//! real capture over a scripted CSSOM and pin the emitted text, which is the only place the
//! failure is visible — an unbalanced result is a declaration the browser drops whole and
//! silently.

use super::{style, walk};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Four hover rules that differ only in which custom property the `var()` names, plus one whose
/// fallback is a quoted string containing a parenthesis.
///
/// `--ring` and `--alt` are seeded as computed values on every element, which is what a `:root`
/// declaration produces for every descendant. `--missing` and `--gone` are declared nowhere.
fn scene() -> Value {
    let sheet = json!([
        style(
            ".defined:hover",
            "box-shadow",
            "0 0 0 2px var(--ring, rgba(0,0,0,.3))"
        ),
        style(
            ".undefined:hover",
            "box-shadow",
            "0 0 0 2px var(--missing, rgba(0,0,0,.3))"
        ),
        style(
            ".nested:hover",
            "box-shadow",
            "0 0 0 2px var(--gone, var(--alt, #fff))"
        ),
        style(
            ".deep:hover",
            "box-shadow",
            "0 0 0 2px var(--gone, var(--ring, rgba(0,0,0,.3)))"
        ),
        style(".quoted:hover", "content", "var(--gone, \")\")")
    ]);
    let computed = json!({ "--ring": "#0f6cbd", "--alt": "#107c10" });
    let element = |class: &str| json!({ "path": format!("/main/{class}"), "classes": [class], "computed": computed });
    let elements: Vec<Value> = ["defined", "undefined", "nested", "deep", "quoted"]
        .iter()
        .map(|class| element(class))
        .collect();
    json!({
        "elements": elements,
        "matching": {},
        "sheets": [sheet]
    })
}

/// The declaration text the capture recorded for each element, keyed by its class.
fn captured() -> BTreeMap<String, String> {
    walk(scene())["stateStyles"]
        .as_array()
        .expect("the walk records state styles")
        .iter()
        .map(|entry| {
            let target = entry["target"].as_str().unwrap();
            (
                target.rsplit('/').next().unwrap().to_string(),
                entry["declarations"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// `(` minus `)`, counting only delimiters outside quoted strings.
fn imbalance(text: &str) -> i32 {
    let mut depth = 0;
    let mut quote = None;
    let mut escaped = false;
    for character in text.chars() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if let Some(open) = quote {
            if character == open {
                quote = None;
            }
        } else {
            match character {
                '"' | '\'' => quote = Some(character),
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }
    }
    depth
}

/// The defect. Every recorded declaration must balance, whatever the substitution did, because
/// CSS discards an unbalanced declaration whole rather than degrading it — so one stray
/// character deletes the entire hover style and nothing anywhere reports it.
#[test]
fn every_substituted_state_declaration_balances_its_parentheses() {
    for (class, declarations) in captured() {
        assert_eq!(
            imbalance(&declarations),
            0,
            "`.{class}` recorded an unbalanced declaration: {declarations}"
        );
    }
}

/// Balance alone is satisfied by recording nothing, and by recording the fallback when the
/// property is defined. Each arm therefore pins the value the browser would have painted.
#[test]
fn each_fallback_arm_resolves_to_the_value_the_engine_would_paint() {
    let captured = captured();
    for (class, expected) in [
        // Defined, with a function-bearing fallback that must be skipped entirely.
        ("defined", "box-shadow: 0 0 0 2px #0f6cbd;"),
        // Undefined, so the fallback wins and must arrive intact.
        ("undefined", "box-shadow: 0 0 0 2px rgba(0,0,0,.3);"),
        // Nested, resolved through the inner fallback.
        ("nested", "box-shadow: 0 0 0 2px #107c10;"),
        // Nested onto a *defined* inner name, which is the arm a widened character class
        // cannot reach: the inner reference has to survive the outer substitution first.
        ("deep", "box-shadow: 0 0 0 2px #0f6cbd;"),
        // A fallback whose parenthesis is string content, not structure.
        ("quoted", "content: \")\";"),
    ] {
        assert_eq!(
            captured.get(class).map(String::as_str),
            Some(expected),
            "`.{class}` was recorded as {:?}",
            captured.get(class)
        );
    }
}
