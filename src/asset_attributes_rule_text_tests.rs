//! What the artifact carries as a rule's text.
//!
//! Collection resolves a reference against the sheet that held it; emission matches the
//! asset map's keys as substrings of that text. The two agree only if the text is spelled
//! the way the map is keyed, and a relative path is not a spelling of a URL — it is a URL
//! plus a base. Resolving here, where the base is known, is what makes the two agree by
//! construction instead of by the emitter guessing at spellings it might recognise.
//!
//! The result is then written in its shortest unambiguous spelling, so the capture rig's
//! ephemeral port never reaches the artifact.

use super::reach_harness::{ORIGIN, rule, rules, walk};
use serde_json::json;

fn leaf() -> serde_json::Value {
    json!({ "tag": "p" })
}

fn resolved(entries: serde_json::Value) -> Vec<String> {
    rules(&walk(&leaf(), &entries, ORIGIN))
}

/// The subject. A bare relative `src` in a subdirectory sheet names a file beside the sheet,
/// not beside the document, and nothing downstream can recover that once the text is flat.
#[test]
fn resolves_a_bare_relative_reference_against_its_own_sheet() {
    let text =
        "@font-face{font-family:subjectface;src:url(\"subjectfont.woff2\") format(\"woff2\");}";
    let resolved = resolved(json!([rule(
        text,
        "http://rig.test:59700/styles/fonts.css"
    )]));
    assert_eq!(
        resolved[0],
        "@font-face{font-family:subjectface;src:url(\"/styles/subjectfont.woff2\") format(\"woff2\");}"
    );
}

/// The control's spelling, and the one the emitter already handles. It must arrive at the
/// same reference, so the two spellings stop being distinguishable downstream.
#[test]
fn resolves_a_root_relative_reference_to_the_same_reference() {
    let bare = resolved(json!([rule(
        "a{src:url(\"controlfont.woff2\")}",
        "http://rig.test:59700/fonts.css"
    )]));
    let rooted = resolved(json!([rule(
        "a{src:url(\"/controlfont.woff2\")}",
        "http://rig.test:59700/styles/fonts.css"
    )]));
    assert_eq!(bare[0], "a{src:url(\"/controlfont.woff2\")}");
    assert_eq!(rooted[0], bare[0]);
}

/// The case any filename-matching shortcut fails. Two sheets in different directories name
/// the same file; they are two different assets and must stay two different references.
#[test]
fn resolves_one_filename_in_two_directories_to_two_assets() {
    let resolved = resolved(json!([
        rule(
            "a{src:url(\"icon.png\")}",
            "http://rig.test:59700/one/a.css"
        ),
        rule(
            "b{src:url(\"icon.png\")}",
            "http://rig.test:59700/two/b.css"
        ),
    ]));
    assert_eq!(resolved[0], "a{src:url(\"/one/icon.png\")}");
    assert_eq!(resolved[1], "b{src:url(\"/two/icon.png\")}");
}

/// The default path, unaffected. A page's own inline `<style>` has no location of its own,
/// so it carries the document base and resolves exactly as it did before.
#[test]
fn resolves_an_inline_rule_against_the_document_base() {
    let resolved = resolved(json!([rule("a{src:url(\"hero.png\")}", ORIGIN)]));
    assert_eq!(resolved[0], "a{src:url(\"/hero.png\")}");
}

/// A reference that is already absolute is already spelled the way the map is keyed, so it
/// must survive untouched rather than be re-based against anything.
#[test]
fn leaves_an_absolute_reference_exactly_where_it_points() {
    let resolved = resolved(json!([rule(
        "a{src:url(\"http://cdn.example/f.woff2\")}",
        "http://rig.test:59700/styles/fonts.css"
    )]));
    assert_eq!(resolved[0], "a{src:url(\"http://cdn.example/f.woff2\")}");
}

/// A data URL carries its own bytes. Resolving it against a base must not move it, and
/// rewriting it must not corrupt the payload.
#[test]
fn leaves_a_data_url_carrying_its_own_bytes() {
    let resolved = resolved(json!([rule(
        "a{src:url(\"data:font/woff2;base64,AAAA\")}",
        "http://rig.test:59700/styles/fonts.css"
    )]));
    assert_eq!(resolved[0], "a{src:url(\"data:font/woff2;base64,AAAA\")}");
}

/// A URL may contain a bracket, and CSSOM's serialize-a-string does not escape one. Written
/// back unquoted it would close the `url()` early and strand the rest of the value, so the
/// rewrite always quotes.
#[test]
fn quotes_a_rewritten_reference_that_contains_a_bracket() {
    let resolved = resolved(json!([rule(
        "a{src:url(\"logo(1).png\")}",
        "http://rig.test:59700/styles/s.css"
    )]));
    assert_eq!(resolved[0], "a{src:url(\"/styles/logo(1).png\")}");
}

/// Text that never mentions a reference is not a candidate for rewriting, and a property
/// whose name merely ends in `url(` is not a reference either.
#[test]
fn leaves_text_that_names_no_reference_untouched() {
    let text = "a{font-family:subjectface;-webkit-mask-box-image-url(x)}";
    let resolved = resolved(json!([rule(text, "http://rig.test:59700/s.css")]));
    assert_eq!(resolved[0], text);
}

/// The hazard resolution creates. A reference the capture cannot download is localised by
/// nothing, so whatever this stage wrote is what ships. Written absolute it would name the
/// capture rig's ephemeral port — a different dead address on every run of the same capture.
/// Origin-relative is both stable and what the reference already meant.
#[test]
fn never_writes_the_capture_origin_into_a_reference() {
    let resolved = resolved(json!([
        rule("a{background-image:url(\"gone.png\")}", ORIGIN),
        rule(
            "b{background-image:url(\"http://rig.test:59700/also-gone.png\")}",
            ORIGIN
        ),
    ]));
    for text in &resolved {
        assert!(
            !text.contains(ORIGIN),
            "reference names the capture rig: {text}"
        );
    }
    assert_eq!(resolved[0], "a{background-image:url(\"/gone.png\")}");
    assert_eq!(resolved[1], "b{background-image:url(\"/also-gone.png\")}");
}

/// Two relative references whose absolute forms stand in a prefix relation. Resolution is
/// what creates the pair, so the emitter's maximal-munch guarantee now has to hold over text
/// this stage produced. `asset_urls_tests::rewrites_two_references_a_sheet_wrote_as_relative`
/// consumes exactly this string, so the two halves of the claim cannot drift apart.
#[test]
fn resolves_two_references_whose_absolute_forms_are_in_a_prefix_relation() {
    let resolved = resolved(json!([rule(
        "@font-face{src:url(\"f.woff2\"),url(\"f.woff\")}",
        "http://rig.test:59700/s/fonts.css"
    )]));
    assert_eq!(
        resolved[0],
        "@font-face{src:url(\"/s/f.woff2\"),url(\"/s/f.woff\")}"
    );
}

#[path = "asset_attributes_division_key_tests.rs"]
mod division_key;
