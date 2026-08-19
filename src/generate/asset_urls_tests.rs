use super::rewrite;
use crate::model::{StateStyle, Styles};
use std::collections::BTreeMap;

/// Two assets whose URLs stand in a prefix relation. A `BTreeMap` iterates
/// byte-lexicographically and a prefix always sorts before the string it prefixes, so an
/// unsorted fold visits `tile.svg` first, consumes it out of `tile.svg.svg`, and strands
/// the tail on the wrong asset's local path.
fn assets() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "http://127.0.0.1:5000/tile.svg".to_string(),
            "/assets/aaaa.svg".to_string(),
        ),
        (
            "http://127.0.0.1:5000/tile.svg.svg".to_string(),
            "/assets/bbbb.svg".to_string(),
        ),
        (
            "http://127.0.0.1:5000/solo.svg".to_string(),
            "/assets/cccc.svg".to_string(),
        ),
    ])
}

/// The composite `/assets/aaaa.svg.svg` — the other asset's local path plus this URL's
/// tail beyond the prefix — is the fingerprint of prefix shadowing. No benign process
/// produces it, so asserting its absence is not the same as asserting the happy path.
const SHADOWED: &str = "/assets/aaaa.svg.svg";

/// The widest path in the emitter: every element's baked computed style flows through it,
/// so every url-bearing property does.
#[test]
fn baked_computed_styles_rewrite_the_longer_asset_url_first() {
    let styles = Styles::from([(
        "background-image".into(),
        r#"url("http://127.0.0.1:5000/tile.svg.svg")"#.into(),
    )]);
    let css = crate::generate::css_values::declarations(&styles, &assets());
    assert!(!css.contains(SHADOWED), "was {css}");
    assert_eq!(css, r#"background-image:url("/assets/bbbb.svg");"#);
}

/// The same guarantee through the hover/focus path, so the two sites cannot drift apart.
#[test]
fn state_styles_rewrite_the_longer_asset_url_first() {
    let style = StateStyle {
        target: "html>body:nth-of-type(1)>div:nth-of-type(1)".into(),
        scope: None,
        relation: Default::default(),
        pseudo: Some(":hover".into()),
        target_pseudo: None,
        media: None,
        declarations: r#"background-image:url("http://127.0.0.1:5000/tile.svg.svg");"#.into(),
    };
    let classes = BTreeMap::from([(style.target.clone(), "r0".to_string())]);
    let mut css = String::new();
    crate::generate::state_styles::append(&[style], &classes, &assets(), &mut css);
    assert!(!css.contains(SHADOWED), "was {css}");
    assert!(css.contains(r#"url("/assets/bbbb.svg")"#), "was {css}");
}

/// Control: an asset in no prefix relation is localised correctly whatever the order, so a
/// failure above is attributable to ordering and not to localisation in general.
#[test]
fn an_asset_in_no_prefix_relation_is_unaffected() {
    let styles = Styles::from([(
        "background-image".into(),
        r#"url("http://127.0.0.1:5000/solo.svg")"#.into(),
    )]);
    assert_eq!(
        crate::generate::css_values::declarations(&styles, &assets()),
        r#"background-image:url("/assets/cccc.svg");"#
    );
}

/// The shorter member of the pair must still be rewritten. A fix that ordered by length
/// but dropped a substitution would pass the two assertions above.
#[test]
fn the_shorter_asset_url_is_still_rewritten() {
    let styles = Styles::from([(
        "background-image".into(),
        r#"url("http://127.0.0.1:5000/tile.svg")"#.into(),
    )]);
    assert_eq!(
        crate::generate::css_values::declarations(&styles, &assets()),
        r#"background-image:url("/assets/aaaa.svg");"#
    );
}

/// The maintainer's original fixture for this hazard, kept verbatim: `font.woff` is a
/// byte-prefix of `font.woff2`, so an unsorted fold strands `2` on the shorter path.
#[test]
fn rewrites_longer_protocol_relative_asset_urls_first() {
    let assets = BTreeMap::from([
        (
            "https://cdn.example/font.woff".to_string(),
            "/assets/font.woff".to_string(),
        ),
        (
            "https://cdn.example/font.woff2".to_string(),
            "/assets/font.woff2".to_string(),
        ),
    ]);
    assert_eq!(
        rewrite(
            r#"src:url("//cdn.example/font.woff2"),url("//cdn.example/font.woff")"#,
            &assets,
        ),
        r#"src:url("/assets/font.woff2"),url("/assets/font.woff")"#
    );
}

#[test]
fn rewrites_a_font_the_stylesheet_wrote_as_a_root_relative_path() {
    let assets = BTreeMap::from([(
        "https://local.example:8080/assets/font/segoe-sans.711fd8a54c.woff2".to_string(),
        "/assets/segoe-sans.woff2".to_string(),
    )]);
    assert_eq!(
        rewrite(
            r#"src:url("/assets/font/segoe-sans.711fd8a54c.woff2") format("woff2")"#,
            &assets,
        ),
        r#"src:url("/assets/segoe-sans.woff2") format("woff2")"#
    );
}

/// A sheet that wrote both references relatively now hands the emitter two absolute URLs in
/// a prefix relation, which only resolution could have produced. The text is exactly what
/// `rule_text_tests::resolves_two_references_whose_absolute_forms_are_in_a_prefix_relation`
/// asserts the capture writes, so maximal munch is proved on the new path, not assumed.
#[test]
fn rewrites_two_references_a_sheet_wrote_as_relative() {
    let assets = BTreeMap::from([
        (
            "http://rig.test:59700/s/f.woff".to_string(),
            "/assets/aaaa.woff".to_string(),
        ),
        (
            "http://rig.test:59700/s/f.woff2".to_string(),
            "/assets/bbbb.woff2".to_string(),
        ),
    ]);
    assert_eq!(
        rewrite(
            r#"@font-face{src:url("/s/f.woff2"),url("/s/f.woff")}"#,
            &assets,
        ),
        r#"@font-face{src:url("/assets/bbbb.woff2"),url("/assets/aaaa.woff")}"#
    );
}

/// A reference whose bytes the capture never obtained has no local path to point at, so it
/// must survive untouched. Rewriting it to anything would name a file the project does not
/// serve while looking repaired.
#[test]
fn leaves_a_reference_the_capture_never_collected() {
    let text = r#"@font-face{src:url("/absent.woff2")}"#;
    assert_eq!(rewrite(text, &assets()), text);
}

/// One owner means one behaviour: the spellings the rule rewriter always handled must now
/// reach the baked path too, which the two unsorted folds never did.
#[test]
fn every_caller_shares_one_set_of_url_spellings() {
    let assets = assets();
    let rule = r#"background-image:url("//127.0.0.1:5000/tile.svg.svg")"#;
    assert_eq!(
        rewrite(rule, &assets),
        r#"background-image:url("/assets/bbbb.svg")"#
    );
    let styles = Styles::from([(
        "background-image".into(),
        r#"url("//127.0.0.1:5000/tile.svg.svg")"#.into(),
    )]);
    assert_eq!(
        crate::generate::css_values::declarations(&styles, &assets),
        r#"background-image:url("/assets/bbbb.svg");"#
    );
}
