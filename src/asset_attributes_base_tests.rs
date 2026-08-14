//! Which base a relative reference is resolved against.
//!
//! HTML resolves a relative reference against the *document base URL*: the first
//! `<base href>`, falling back to the document's own URL when there is none.
//! `location.href` is the document's *location* and never sees `<base>`. The two are
//! byte-identical on every page without a `<base href>` — which is every other fixture in
//! this crate — so nothing here can be reproduced by widening an existing test. The input
//! that separates them has to be constructed, and that is what this file is.
//!
//! The failure that follows from the wrong operand is not a loss but a fabrication. A
//! reference the page painted from `nested/hero.png` is recorded as `hero.png` resolved
//! against the location: well-formed, absolute, and never requested. It therefore 404s,
//! never enters the asset map, and survives into the artifact as a live address on the
//! capture rig's ephemeral port.
//!
//! Both halves are asserted throughout, because `resolveUrl` is used twice and a repair to
//! either alone is silent: `recreateAssetUrls` decides which bytes are downloaded, and
//! `recreateAttributes` decides what the recorded attribute says. Agreement between them
//! proves nothing — they consume the same operand, so they agree while both are wrong.

use serde_json::{Value, json};

use super::reach_harness::{LOCATION, walk};

const BASED: &str = "http://rig.test:59700/nested/";

/// Everything one asset-bearing element produced: the URLs collected for download, and the
/// attribute values recorded onto the node.
fn walked(attributes: Value, rules: Value, base: &str) -> (Vec<String>, Value) {
    let tree = json!({ "tag": "img", "assetBearing": true, "attributes": attributes });
    let result = walk(&tree, &rules, base);
    (
        super::reach_harness::assets(&result),
        result["recorded"][0].clone(),
    )
}

/// One attribute on one element, under a document base that differs from the location.
fn based(name: &str, value: &str) -> (Vec<String>, String) {
    let (assets, recorded) = walked(json!({ name: value }), json!([]), BASED);
    (
        assets,
        recorded[name].as_str().unwrap_or_default().to_string(),
    )
}

/// The subject. A page at `/page.html` with `<base href="nested/">` paints
/// `<img src="basetoken.png">` from `/nested/basetoken.png`, so that is the only address
/// the capture may record. Resolving against the location instead yields `/basetoken.png`
/// — a sibling of the page that the page never requested and that need not exist.
#[test]
fn a_relative_reference_resolves_against_the_document_base_not_the_location() {
    let (assets, recorded) = based("src", "basetoken.png");
    assert_eq!(assets, vec!["http://rig.test:59700/nested/basetoken.png"]);
    assert_eq!(recorded, "http://rig.test:59700/nested/basetoken.png");
}

/// The two uses of the resolver are asserted separately above; this pins why. The recorded
/// attribute is overwritten with the resolved string, so a wrong operand does not merely
/// mis-key the download — it rewrites the element to point somewhere the page never did,
/// and the emitter's exact lookup then misses and passes the fabrication straight through.
#[test]
fn the_collected_url_and_the_recorded_attribute_are_the_same_address() {
    let (assets, recorded) = based("src", "deep/token.png");
    assert_eq!(assets, vec![recorded.clone()]);
    assert_eq!(recorded, "http://rig.test:59700/nested/deep/token.png");
}

/// The control, and the reason the scene's second `<img>` is immune by construction: a
/// root-relative reference discards the base's path entirely, so both operands agree. If
/// this ever diverged, the subject's failure could not be attributed to the operand.
#[test]
fn a_root_relative_reference_is_unaffected_by_the_document_base() {
    let (assets, recorded) = based("src", "/controltoken.png");
    assert_eq!(assets, vec!["http://rig.test:59700/controltoken.png"]);
    assert_eq!(recorded, "http://rig.test:59700/controltoken.png");
}

/// A base may point at another origin — `<base href="https://cdn.example/app/">` is the
/// shape a page uses to host its assets elsewhere. Nothing may assume the base shares the
/// location's origin, so the divergence is asserted across one.
#[test]
fn a_document_base_on_another_origin_carries_the_reference_with_it() {
    let (assets, recorded) = walked(
        json!({ "src": "hero.png" }),
        json!([]),
        "https://cdn.example/app/",
    );
    assert_eq!(assets, vec!["https://cdn.example/app/hero.png"]);
    assert_eq!(recorded["src"], "https://cdn.example/app/hero.png");
}

/// Inverse guard, and the whole reason this change is safe to land on a corpus: with no
/// `<base href>` the document base *is* the document URL, so the operands are equal and
/// every existing fixture and scene is untouched. This is also the anti-vacuity twin of
/// the subject — it must keep the location's directory, or the tests above would pass for
/// a resolver that had simply started ignoring its base.
#[test]
fn a_page_without_a_base_resolves_exactly_as_it_did_against_the_location() {
    let (assets, recorded) = walked(json!({ "src": "token.png" }), json!([]), LOCATION);
    assert_eq!(assets, vec!["http://rig.test:59700/token.png"]);
    assert_eq!(recorded["src"], "http://rig.test:59700/token.png");
}

/// Absolute references ignore any base, per the URL parser. Asserted under a base that
/// would otherwise be visible, so the assertion is about the reference's form and not
/// about the base being absent.
#[test]
fn an_absolute_reference_ignores_the_document_base() {
    let (assets, recorded) = based("src", "https://elsewhere.test/fixed.png");
    assert_eq!(assets, vec!["https://elsewhere.test/fixed.png"]);
    assert_eq!(recorded, "https://elsewhere.test/fixed.png");
}

/// A `data:` URL carries its own bytes and has no base to resolve against. It must survive
/// byte-identical rather than be reparsed into an origin-prefixed string, and it must not
/// be queued for download.
#[test]
fn a_data_reference_is_recorded_verbatim_and_never_queued() {
    let data = "data:image/gif;base64,R0lGODlhAQABAAAAACw=";
    let (assets, recorded) = based("src", data);
    assert_eq!(recorded, data);
    assert!(
        !assets.iter().any(|url| url.starts_with("http")),
        "{assets:?}"
    );
}

/// Every candidate in a descriptor list is a separate reference and each is resolved
/// separately, so the base has to reach all of them rather than the first. The descriptors
/// must survive, because they are the reader's only record of which candidate is which.
#[test]
fn every_candidate_in_a_descriptor_list_is_resolved_against_the_document_base() {
    let (assets, recorded) = based("srcset", "small.png 1x, deep/large.png 2x");
    assert_eq!(
        recorded,
        "http://rig.test:59700/nested/small.png 1x, http://rig.test:59700/nested/deep/large.png 2x"
    );
    assert_eq!(
        assets,
        vec![
            "http://rig.test:59700/nested/deep/large.png",
            "http://rig.test:59700/nested/small.png"
        ]
    );
}

/// The resolver's other caller. `cssRules` carries `cssText`, and CSSOM's *serialize a URL*
/// reproduces the authored spelling rather than a resolved one, so a relative `url()`
/// arrives here still relative and is resolved by this same line. Measured, not assumed:
/// a capture of an inline sheet holding `url(basetoken2.png)` recorded exactly that text.
#[test]
fn a_relative_url_in_a_recorded_rule_resolves_against_the_document_base() {
    let (assets, _) = walked(
        json!({}),
        json!([r#".a { background-image: url("basetoken2.png"); }"#]),
        BASED,
    );
    assert_eq!(assets, vec!["http://rig.test:59700/nested/basetoken2.png"]);
}

/// The collector reads URLs from four places — attributes, a node's style map, a generated
/// box's style and its `content` — and the base is a property of the page, not of the field
/// a URL happened to arrive in. Today the style paths are fed computed values, which are
/// already absolute and so cannot show which operand was used; that is exactly why it is
/// asserted here rather than left to be inferred. A caller that starts supplying authored
/// values, or an engine that serialises one relative, must not silently resolve it against
/// the location.
#[test]
fn a_relative_url_in_a_node_style_resolves_against_the_document_base() {
    let tree = json!({
        "tag": "div",
        "style": { "background-image": r#"url("panel.png")"# },
        "pseudos": { "::before": { "style": {}, "content": r#"url("mark.png")"# } }
    });
    let result = super::reach_harness::walk(&tree, &json!([]), BASED);
    assert_eq!(
        super::reach_harness::assets(&result),
        vec![
            "http://rig.test:59700/nested/mark.png",
            "http://rig.test:59700/nested/panel.png"
        ]
    );
}
