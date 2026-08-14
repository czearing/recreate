//! Recovering a stylesheet the page could not read for itself.
//!
//! A cross-origin sheet's rules arrive as text with none of the sheet around them, so what
//! the sheet carried and its text does not — its `media` condition — has to be joined back
//! to it. The only identity both sides hold is the sheet's own address.

use super::{recorded as recorded_rules, style, walk};
use serde_json::{Value, json};

const CDN: &str = "https://cdn.example/print.css";
const TEXT: &str = ".attr{color:green}";
const WRAPPED: &str = "@media print{.attr { color: green; }}";

fn recovered(sheets: Value, authored: Value) -> Vec<String> {
    recorded_rules(&walked(sheets, authored))
}

fn walked(sheets: Value, authored: Value) -> Value {
    walk(json!({
        "elements": [{ "path": "/attr", "classes": ["attr"] }],
        "matching": {},
        "sheets": sheets,
        "authoredSheets": authored,
        "parsed": { TEXT: [style(".attr", "color", "green")] }
    }))
}

fn text_for(href: &str) -> Value {
    json!([{ "text": TEXT, "href": href }])
}

/// The `media` attribute of the `<link>` that fetched a sheet is not in the sheet's text and
/// never can be, so recovered rules inherit it from the sheet the text belongs to. Without
/// the join, a cross-origin print stylesheet — which is how print styles are usually served
/// — is republished as though it applied on screen.
#[test]
fn a_recovered_sheet_inherits_the_condition_of_the_link_that_fetched_it() {
    let recorded = recovered(
        json!([{ "href": CDN, "media": "print", "unreadable": true, "rules": [] }]),
        text_for(CDN),
    );
    assert_eq!(recorded, vec![WRAPPED]);
}

/// Recovery must not run for a sheet already walked. `recordRule` keys on exact rule text,
/// so an unconditioned second copy of a conditioned rule is a different key and no
/// deduplication can collapse the pair: the fabricated rule would survive beside its own
/// repair, and seeding the condition would read as having done nothing.
#[test]
fn a_sheet_already_read_is_not_walked_again_from_its_text() {
    let recorded = recovered(
        json!([{ "href": CDN, "media": "print", "rules": [style(".attr", "color", "green")] }]),
        text_for(CDN),
    );
    assert_eq!(recorded, vec![WRAPPED]);
}

/// Every `<style>` element, document-written sheet and constructed sheet serialises under
/// the document's own address rather than one of its own, and none of them can be unreadable
/// — they are same-origin by construction. Text arriving under an address no document sheet
/// claims is therefore a copy of rules already collected, which on a real page includes the
/// stylesheets of every subframe.
#[test]
fn text_for_a_sheet_no_document_sheet_claims_is_ignored() {
    let recorded = recovered(
        json!([{ "media": "print", "rules": [style(".attr", "color", "green")] }]),
        text_for("https://page.example/"),
    );
    assert_eq!(recorded, vec![WRAPPED]);
}

/// The recovery itself still has to work: an unreadable sheet with no condition contributes
/// its rules, which is the whole reason the fallback exists. Without this, dropping the
/// fallback entirely would pass every other test in this file.
#[test]
fn an_unreadable_sheet_still_contributes_its_rules() {
    let recorded = recovered(
        json!([{ "href": CDN, "unreadable": true, "rules": [] }]),
        text_for(CDN),
    );
    assert_eq!(recorded, vec![".attr { color: green; }"]);
}

/// Recovery is not free: re-parsing a sheet is the second full CSS parse of the same bytes,
/// once per external sheet on every capture. Dedup hides the duplicate rules but not the
/// work, so the guard is asserted where it is visible — the sheet the page could read is
/// never handed to the parser at all.
#[test]
fn a_sheet_the_page_could_read_is_never_parsed_a_second_time() {
    let readable =
        json!([{ "href": CDN, "media": "print", "rules": [style(".attr", "color", "green")] }]);
    assert_eq!(walked(readable, text_for(CDN))["parses"], 0);
}

/// Anti-vacuity for the count above: the same measurement reports one parse when the sheet
/// genuinely could not be read, so a zero there means a decision rather than a broken probe.
#[test]
fn an_unreadable_sheet_is_parsed_exactly_once() {
    let unreadable = json!([{ "href": CDN, "unreadable": true, "rules": [] }]);
    assert_eq!(walked(unreadable, text_for(CDN))["parses"], 1);
}
