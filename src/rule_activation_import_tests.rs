//! A stylesheet reached by reference rather than by ownership.
//!
//! CSSOM builds `document.styleSheets` from owner *nodes*, so an `@import`ed sheet — which
//! has an owner *rule* and no owner node — appears in no collection the walk enumerates.
//! `CSSImportRule` is not a `CSSGroupingRule` either, so the walk pushed it as a leaf and
//! never read `.styleSheet`. Both halves of the pipeline believed the other had it:
//! `generate/css.rs` discards `@import` because "the capture already walked and baked" the
//! sheet it names, which was never true.
//!
//! Ordinary rules inside such a sheet survive anyway, baked into per-element classes, so
//! the loss is invisible until the sheet holds an at-rule that *defines* a name. Then the
//! reference survives in the baked declaration and the definition does not.

use super::{recorded as recorded_rules, style, walk};
use serde_json::{Value, json};

const KEYFRAMES: &str = "@keyframes pulse { from { opacity: 0.25; } }";

fn definition() -> Value {
    json!({
        "prelude": "@keyframes pulse",
        "keyframes": true,
        "rules": [style("from", "opacity", "0.25")]
    })
}

fn walked(scene: Value) -> Value {
    let mut base = json!({
        "elements": [{ "path": "/box", "classes": ["box"] }],
        "matching": {}
    });
    for (key, value) in scene.as_object().unwrap() {
        base[key] = value.clone();
    }
    walk(base)
}

fn rules(sheets: Value) -> Vec<String> {
    recorded_rules(&walked(json!({ "sheets": sheets })))
}

/// The subject. The definition lives only in the imported sheet, and a computed style
/// carries the name it is referred to by and nothing of the block itself, so a walk that
/// cannot enter the sheet leaves every `animation-name` in the page dangling.
#[test]
fn a_definition_in_an_imported_sheet_reaches_the_record() {
    let recorded = rules(json!([[{ "import": { "rules": [definition()] } }]]));
    assert!(
        recorded.contains(&KEYFRAMES.to_string()),
        "an imported sheet was never walked: {recorded:?}"
    );
}

/// The twin relation, naming no literal: where an author writes the definition cannot
/// change what is recorded, because `@import` is defined to behave as though the imported
/// sheet's contents were written in place of the rule.
#[test]
fn the_imported_route_records_what_the_inline_route_records() {
    let imported = rules(json!([[{ "import": { "rules": [definition()] } }]]));
    let inline = rules(json!([[definition()]]));
    assert_eq!(imported, inline, "the two routes disagree");
    assert!(!inline.is_empty(), "neither route recorded anything");
}

/// The import rule is consumed, not recorded. Re-emitting `@import` would refetch the sheet
/// and apply every rule in it a second time, which is exactly why `generate/css.rs` discards
/// it — so the walk must not hand that text downstream and rely on a later stage to drop it.
#[test]
fn the_import_rule_itself_is_not_recorded_as_an_authored_rule() {
    let recorded = rules(json!([[{ "import": { "rules": [definition()] } }]]));
    assert!(
        !recorded.iter().any(|rule| rule.contains("@import")),
        "recorded the import statement itself: {recorded:?}"
    );
}

/// `CSSImportRule.media` is defined to return the *imported sheet's* own `media`, so the
/// query trailing the prelude arrives without the prelude ever being parsed — the sheet
/// condition machinery that already serves `<style media>` and `<link media>` covers it.
#[test]
fn the_query_trailing_an_import_prelude_conditions_the_rules_it_admits() {
    let recorded = rules(json!([[{
        "import": { "media": "print", "rules": [style(".box", "color", "green")] }
    }]]));
    assert_eq!(recorded, vec!["@media print{.box { color: green; }}"]);
}

/// A declaration applies only when the medium matches "on all links on the path through
/// which the style sheet was reached" (CSS 2.1 §6.4.1), so an import nested under a
/// conditioned sheet composes rather than replaces. A descent re-seeded from scratch at each
/// level would publish rules the cascade withheld.
#[test]
fn an_import_composes_its_condition_with_the_sheet_that_holds_it() {
    let recorded = rules(json!([{
        "media": "screen",
        "rules": [{ "import": { "media": "print", "rules": [style(".box", "color", "green")] } }]
    }]));
    assert_eq!(
        recorded,
        vec!["@media screen{@media print{.box { color: green; }}}"]
    );
}

/// A null `styleSheet` is an answer rather than a failure: CSSOM requires it when a
/// `supports()` condition blocked the fetch, and the user agent "must not fetch the style
/// sheet". Nothing was ever loaded, so recording nothing is correct — and the walk must
/// survive to record everything after it.
#[test]
fn an_import_the_agent_never_fetched_contributes_nothing_and_stops_nothing() {
    let recorded = rules(json!([[
        { "import": null },
        style(".box", "color", "red")
    ]]));
    assert_eq!(recorded, vec![".box { color: red; }"]);
}

/// Nothing in CSSOM bounds the import graph, and a walk that relies on the browser breaking
/// the fetch cycle relies on an unwritten guarantee. Asserted as work rather than as output,
/// because an unbounded descent does not crash: the recursion unwinds into the same catch
/// that guards an unreadable sheet, so the rules still arrive and only the cost betrays it.
#[test]
fn a_sheet_that_imports_itself_is_walked_once_rather_than_forever() {
    let result = walked(json!({
        "sheets": [[{ "import": "a" }]],
        "named": { "a": { "rules": [{ "import": "a" }, style(".box", "color", "red")] } }
    }));
    assert_eq!(recorded_rules(&result), vec![".box { color: red; }"]);
    assert_eq!(
        result["reads"], 2,
        "the cycle was re-entered: the document sheet and the imported one, once each"
    );
}

/// Two imports of one address are two sheets, and both carry their own rules. This is the
/// direction an address-keyed bound gets wrong, so it is asserted beside the cycle it would
/// otherwise appear to fix — and it is the anti-vacuity case for the count above, which a
/// walk that had simply stopped entering sheets would also satisfy.
#[test]
fn two_imports_of_one_address_both_contribute() {
    let result = walked(json!({ "sheets": [[
        { "import": { "href": "shared.css", "rules": [style(".box", "color", "red")] } },
        { "import": { "href": "shared.css", "media": "print", "rules": [definition()] } }
    ]] }));
    let recorded = recorded_rules(&result);
    assert!(
        recorded.contains(&".box { color: red; }".to_string()),
        "{recorded:?}"
    );
    assert!(
        recorded.contains(&format!("@media print{{{KEYFRAMES}}}")),
        "the second import of the same address was discarded: {recorded:?}"
    );
    assert_eq!(
        result["reads"], 3,
        "expected the document sheet and both imports"
    );
}

/// The most common `@import` on the web points at a font service, and a cross-origin sheet
/// throws `SecurityError` on `cssRules`. Its text is recoverable through the browser's own
/// CSSOM, but only for a sheet the walk registered as still owed its rules — so the set of
/// pending sheets has to be a superset of what the walk enters, by construction rather than
/// by coincidence, or the definitions that arrive only that way are lost twice over.
#[test]
fn an_imported_sheet_the_page_cannot_read_is_recovered_from_its_text() {
    let recorded = recorded_rules(&walked(json!({
        "sheets": [[{ "import": { "href": "https://cdn.example/f.css", "unreadable": true } }]],
        "authoredSheets": [{ "text": "K", "href": "https://cdn.example/f.css" }],
        "parsed": { "K": [definition()] }
    })));
    assert_eq!(recorded, vec![KEYFRAMES]);
}

/// The inverse guard. Every fixture in the suite is a sheet holding no import, and the
/// repair must be a no-op for all of them.
#[test]
fn a_sheet_holding_no_import_is_recorded_exactly_as_before() {
    assert_eq!(
        rules(json!([[style(".box", "color", "red")]])),
        vec![".box { color: red; }"]
    );
}
