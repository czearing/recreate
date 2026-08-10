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
