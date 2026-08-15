//! Whether a recorded rule still knows which sheet produced it.
//!
//! Everything downstream that resolves a relative `url()` needs the base of the sheet the
//! rule came from, and CSS 2.1 §4.3.4 says that base is the sheet's own and not the
//! document's. The sheet is an object while the walk is inside it and unrecoverable once its
//! rules have been flattened to text, so the association has to be recorded here or nowhere.
//!
//! These assertions are about the carrier, not about any URL: a rule that arrives downstream
//! carrying the wrong base is indistinguishable from one carrying none, and both make every
//! address a guess.

use serde_json::{Value, json};

use super::{style, walk};

/// The document base the harness reports, which is what a sheet with no location inherits.
const DOCUMENT: &str = "http://harness.test/index.html";

fn based(result: &Value) -> Vec<(String, String)> {
    result["cssRules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| {
            (
                rule["text"].as_str().unwrap().to_string(),
                rule["base"].as_str().unwrap_or("<none>").to_string(),
            )
        })
        .collect()
}

fn base_of(result: &Value, needle: &str) -> String {
    let rules = based(result);
    let found: Vec<_> = rules
        .iter()
        .filter(|(text, _)| text.contains(needle))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected one rule matching {needle}: {rules:?}"
    );
    found[0].1.clone()
}

fn scene(sheets: Value) -> Value {
    json!({ "elements": [], "matching": {}, "sheets": sheets })
}

/// The subject. A sheet loaded from a subdirectory is the base for every rule inside it, so
/// the location the walk already holds has to travel with the text it flattens.
#[test]
fn a_rule_is_recorded_with_the_location_of_the_sheet_that_held_it() {
    let result = walk(scene(json!([
        { "href": "http://rig.test/styles/subsheet.css", "rules": [style(".sub", "color", "red")] },
        { "href": "http://rig.test/rootsheet.css", "rules": [style(".root", "color", "blue")] }
    ])));
    assert_eq!(
        base_of(&result, ".sub"),
        "http://rig.test/styles/subsheet.css"
    );
    assert_eq!(base_of(&result, ".root"), "http://rig.test/rootsheet.css");
}

/// A `<style>` element has no location, and CSSOM returns null for its `href`. Its base is
/// the document's, so the answer that was correct before this distinction existed has to fall
/// out of the default rather than sit beside it in a branch.
#[test]
fn a_sheet_with_no_location_carries_the_document_base() {
    let result = walk(scene(json!([[style(".inline", "color", "red")]])));
    assert_eq!(base_of(&result, ".inline"), DOCUMENT);
}

/// `CSSImportRule.styleSheet.href` is already resolved against the sheet that imported it, so
/// an import chain needs no accumulator — but only if each sheet stamps its own rules rather
/// than the outermost stamping everything it collected. Two levels, because one level cannot
/// tell "the innermost sheet won" from "the only sheet won".
#[test]
fn an_imported_sheet_stamps_its_own_location_not_its_importers() {
    let result = walk(scene(json!([{
        "href": "http://rig.test/rootsheet.css",
        "rules": [
            style(".root", "color", "blue"),
            { "import": {
                "href": "http://rig.test/styles/imported.css",
                "rules": [
                    style(".mid", "color", "green"),
                    { "import": {
                        "href": "http://rig.test/styles/deep/nested.css",
                        "rules": [style(".deep", "color", "red")]
                    }}
                ]
            }}
        ]
    }])));
    assert_eq!(base_of(&result, ".root"), "http://rig.test/rootsheet.css");
    assert_eq!(
        base_of(&result, ".mid"),
        "http://rig.test/styles/imported.css"
    );
    assert_eq!(
        base_of(&result, ".deep"),
        "http://rig.test/styles/deep/nested.css"
    );
}

/// A sheet the page could not read is recovered from its text through a constructed
/// `CSSStyleSheet`, whose own `href` is null. Taking the base from that object would silently
/// re-document-base every cross-origin sheet on the web — the shape most `@import`s have. The
/// address it was recovered under is the one that must be stamped.
#[test]
fn a_recovered_sheet_is_stamped_with_the_address_it_was_recovered_under() {
    let text = ".recovered { color: red; }";
    let mut scene = scene(json!([
        { "href": "http://rig.test/styles/blocked.css", "unreadable": true, "rules": [] }
    ]));
    scene["authoredSheets"] =
        json!([{ "text": text, "href": "http://rig.test/styles/blocked.css" }]);
    scene["parsed"] = json!({ text: [style(".recovered", "color", "red")] });
    let result = walk(scene);
    assert_eq!(
        base_of(&result, ".recovered"),
        "http://rig.test/styles/blocked.css"
    );
}
