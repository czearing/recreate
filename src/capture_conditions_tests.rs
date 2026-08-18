//! Which properties an authored condition decided, read from the engine rather than the text.
//!
//! Every case here runs the shipped pass over a scripted CSSOM whose conditions the scene
//! answers, exactly as a viewport and a container's used size answer them in a browser.

use super::{style, walk};
use serde_json::{Value, json};

/// One element under one `@container` that holds, with the override spelled in a vocabulary
/// no computed sample uses, plus an element under a `@supports` that also holds.
fn scene() -> Value {
    json!({
        "elements": [
            { "path": "/main/p", "classes": ["arm"] },
            { "path": "/main/span", "classes": ["grid"] }
        ],
        "matching": {
            "@container (max-width: 400px)": ["/main/p"],
            "@supports (display: grid)": ["/main/span"]
        },
        "sheets": [[
            style(".arm", "padding-left", "42px"),
            style(".grid", "display", "block"),
            {
                "prelude": "@container (max-width: 400px)",
                "conditionText": "(max-width: 400px)",
                "rules": [style(".arm", "padding-left", "0.5em")]
            },
            {
                "prelude": "@supports (display: grid)",
                "conditionText": "(display: grid)",
                "rules": [style(".grid", "display", "grid")]
            },
            {
                "prelude": "@media (min-width: 1px)",
                "conditionText": "(min-width: 1px)",
                "rules": [{
                    "prelude": "@position-try --shift",
                    "descriptors": true,
                    "declarations": { "top": "anchor(bottom)", "width": "auto" }
                }]
            }
        ]]
    })
}

fn decided(result: &Value, path: &str) -> Vec<String> {
    result["decided"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["path"] == path)
        .unwrap()["condition_decided"]
        .as_array()
        .map(|names| {
            names
                .iter()
                .map(|name| name.as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// The filed defect's acquisition half. `0.5em` is never the string a computed sample holds,
/// so no comparison of the two can report that a condition decided this property — and the
/// engine reports it without being told what an `em` is.
#[test]
fn names_a_property_an_override_decided_however_it_was_spelled() {
    assert_eq!(decided(&walk(scene()), "/main/p"), ["padding-left"]);
}

/// `@supports` is answered by the engine, not the document, so the recreation bakes its
/// branch in rather than re-emitting it. Reporting it as condition-decided would let a later
/// stage withdraw a value nothing puts back — the reason the pass follows the carrier split
/// rather than withdrawing every conditional rule.
#[test]
fn leaves_a_gate_the_recreation_does_not_re_emit_out_of_the_answer() {
    assert!(decided(&walk(scene()), "/main/span").is_empty());
}

/// The pass reads the page and must leave it as it found it, or every stage after it — and
/// the page itself, which is still live — sees a document the source never had.
#[test]
fn puts_every_withdrawn_block_back() {
    let result = walk(scene());
    let blocks: Vec<_> = result["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|block| block["cssText"].as_str().unwrap().to_string())
        .collect();

    assert!(
        blocks
            .iter()
            .any(|text| text.contains("padding-left: 0.5em")),
        "a withdrawn block was left empty: {blocks:?}"
    );
    assert!(
        blocks.iter().all(|text| !text.is_empty()),
        "a withdrawn block was left empty: {blocks:?}"
    );
}

/// The anti-vacuity control. A condition that does not hold decides nothing, so an answer
/// that named every property of every conditional rule would fail here.
#[test]
fn names_nothing_for_a_condition_that_does_not_hold() {
    let mut scene = scene();
    scene["matching"] = json!({ "@supports (display: grid)": ["/main/span"] });

    assert!(decided(&walk(scene), "/main/p").is_empty());
}

/// Observed on github.com: a `@position-try` nested in a `@media` carries a declaration
/// block whose names are descriptors, and Chrome reports its `length` without exposing
/// them. A rule with no selector reaches no element, so it can have decided nothing — the
/// same line `activateEntries` already draws, which covers every definition rule at once
/// rather than the one that happened to be found.
#[test]
fn passes_over_a_definition_rule_that_selects_nothing() {
    let mut scene = scene();
    scene["matching"]["@media (min-width: 1px)"] = json!(["/main/p", "/main/span"]);

    assert_eq!(decided(&walk(scene), "/main/p"), ["padding-left"]);
}

/// The blocks are emptied with their sheets switched off, because a live sheet rebuilds its
/// rule data on every declaration handed to it. A sheet the page had switched off itself
/// must be found switched off after, or the recreation captures a document the source never
/// showed — and one still on must be on, or every later read measures a stripped page.
#[test]
fn leaves_every_sheet_switched_as_the_page_had_it() {
    let mut scene = scene();
    let sheet = scene["sheets"][0].take();
    scene["sheets"] = json!([
        { "rules": sheet },
        {
            "disabled": true,
            "rules": [{
                "prelude": "@container (max-width: 400px)",
                "conditionText": "(max-width: 400px)",
                "rules": [style(".arm", "padding-left", "99px")]
            }]
        }
    ]);
    let result = walk(scene);

    assert_eq!(
        result["switches"],
        json!([false, true]),
        "{:?}",
        result["switches"]
    );
}
