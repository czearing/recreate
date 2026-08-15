//! Which base a `url()` inside a stylesheet rule is resolved against.
//!
//! CSS 2.1 §4.3.4 gives the answer, and it is not the one HTML gives for an attribute:
//! "For CSS style sheets, the base URI is that of the style sheet, not that of the source
//! document." Its worked example is exact — `body { background: url("yellow") }` inside
//! `http://www.example.org/style/basic.css` designates `http://www.example.org/style/yellow`.
//!
//! `base_tests` covers the other operand and cannot see this one: every rule it supplies
//! comes from a sheet whose own location is the document's, which is true by definition for
//! an inline `<style>` and true by accident for a sheet at the document root. The input that
//! separates them is a sheet in a subdirectory, and that is what this file is.
//!
//! The failure is silent for almost every rule, because a rule some captured element matches
//! also reaches the collector through the computed-style loop, where the engine has already
//! resolved the address. The guess is merely added alongside the correct answer and costs a
//! wasted 404. It becomes the *sole* address only when nothing captured matches the rule —
//! and such rules are recorded in full, so the asset is named, mis-addressed, and lost.

use serde_json::{Value, json};

use super::reach_harness::{ORIGIN, rule, walk};

/// A sheet one directory below the document. Its rules' base is this, not `DOCUMENT`.
const SHEET: &str = "http://rig.test:59700/styles/subsheet.css";
/// The document base, which is also the base of any inline sheet the page carries.
const DOCUMENT: &str = "http://rig.test:59700/page.html";

fn assets(rules: Value, document_base: &str) -> Vec<String> {
    super::reach_harness::assets(&walk(&json!({ "tag": "div" }), &rules, document_base))
}

/// The subject. `styles/subsheet.css` names `unmatchedsub.png` beside itself, so the only
/// address that ever existed is `/styles/unmatchedsub.png`. Resolving against the document
/// instead yields `/unmatchedsub.png` — well-formed, absolute, never served, never shipped.
#[test]
fn a_relative_url_in_a_subdirectory_sheet_resolves_against_that_sheet() {
    assert_eq!(
        assets(
            json!([rule(
                ".unmatchedsubtoken { background-image: url(unmatchedsub.png); }",
                SHEET
            )]),
            DOCUMENT
        ),
        vec!["http://rig.test:59700/styles/unmatchedsub.png"]
    );
}

/// The inverse guard, and the reason the corpus is untouched: for a sheet at the document
/// root the two bases name the same directory, so the fix must be a no-op there. If this
/// ever moved, the repair would have started resolving against the sheet *file* rather than
/// the directory it sits in.
#[test]
fn a_root_level_sheet_resolves_exactly_where_the_document_does() {
    assert_eq!(
        assets(
            json!([rule(
                ".unmatchedroottoken { background-image: url(unmatchedroot.png); }",
                "http://rig.test:59700/rootsheet.css"
            )]),
            DOCUMENT
        ),
        vec!["http://rig.test:59700/unmatchedroot.png"]
    );
}

/// The two operands simultaneously live and different, which is the case neither file could
/// assert alone: a `<base href>` moves the document base away from the location, and an
/// external sheet in a subdirectory has a base that is neither. One rule from each, in one
/// walk, and both must land where their own carrier says.
#[test]
fn an_inline_sheet_under_a_base_href_and_a_subdirectory_sheet_both_resolve_correctly() {
    let based = "http://rig.test:59700/nested/";
    assert_eq!(
        assets(
            json!([
                rule(".inline { background-image: url(inlinetoken.png); }", based),
                rule(".sub { background-image: url(subtoken.png); }", SHEET)
            ]),
            based
        ),
        vec![
            "http://rig.test:59700/nested/inlinetoken.png",
            "http://rig.test:59700/styles/subtoken.png"
        ]
    );
}

/// A rule's base governs only the rule. The other three places the collector reads URLs from
/// — attributes, a node's style map and a generated box — are properties of the document, and
/// a repair that started basing them on a sheet URL would break every element that carries a
/// relative reference. Asserted with a sheet base that would be visible if it leaked.
#[test]
fn a_sheet_base_never_reaches_an_element_reference() {
    let tree = json!({
        "tag": "img",
        "assetBearing": true,
        "attributes": { "src": "hero.png" }
    });
    let result = walk(
        &tree,
        &json!([rule(".a { background-image: url(sub.png); }", SHEET)]),
        DOCUMENT,
    );
    assert_eq!(
        super::reach_harness::assets(&result),
        vec![
            format!("{ORIGIN}hero.png"),
            format!("{ORIGIN}styles/sub.png")
        ]
    );
}

/// An absolute reference ignores every base, and a `data:` URL has none to ignore. Both are
/// asserted under a sheet base that would otherwise be visible, so the assertion is about the
/// reference's form rather than about the base being absent.
#[test]
fn an_absolute_or_data_url_in_a_sheet_rule_ignores_the_sheet_base() {
    let assets = assets(
        json!([rule(
            ".a { background: url(https://elsewhere.test/fixed.png); content: url(data:image/gif;base64,R0lGODlhAQABAAAAACw=); }",
            SHEET
        )]),
        DOCUMENT,
    );
    assert_eq!(assets, vec!["https://elsewhere.test/fixed.png"]);
}
