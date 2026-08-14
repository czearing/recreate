//! A generated box's `content` carries asset URLs like any other declaration, and the
//! capture reads it back absolutised against the rig's ephemeral origin. These tests pin
//! that the emitted rule and the class identity both spell it the way the recreation
//! serves it, because the two must agree and neither may name the rig.

use super::css_pseudo_identity_tests::{classes_with_assets, span};
use crate::model::{Pseudo, Styles};
use std::collections::BTreeMap;

const RIG: &str = "http://localhost:59898/mark.png";
const LOCAL: &str = "/assets/4c0867e7fadfde14dfb2.png";

fn assets() -> BTreeMap<String, String> {
    BTreeMap::from([(RIG.into(), LOCAL.into())])
}

/// A generated box whose `content` names an asset, spelled as the capture records it. The
/// same value is in the style map, which is how the bytes were downloaded in the first place.
fn marked(ordinal: usize, content: &str) -> crate::model::Node {
    let mut style = Styles::new();
    style.insert("background-image".into(), format!("url(\"{RIG}\")"));
    style.insert("content".into(), content.into());
    let mut node = span(ordinal);
    node.pseudos.insert(
        "::before".into(),
        Pseudo {
            content: content.into(),
            style,
        },
    );
    node
}

/// The subject. The recreation must serve its own bytes; a rule naming the capture rig
/// paints nothing once that rig is gone.
#[test]
fn localises_an_asset_url_in_generated_content() {
    let (_, css) = classes_with_assets(vec![marked(1, &format!("url(\"{RIG}\")"))], &assets());

    assert!(
        css.contains(&format!("content:url(\"{LOCAL}\")")),
        "generated content still names the capture rig: {css}"
    );
    assert!(
        !css.contains(RIG),
        "the rig's origin survived into the emitted rule: {css}"
    );
}

/// The twin relation the filing states: two url-valued declarations on one generated box,
/// one travelling the `content` field and one the style map, must resolve alike. Stated
/// without naming a path, so it survives any change to how assets are addressed.
#[test]
fn content_and_background_resolve_to_the_same_asset_root() {
    let (_, css) = classes_with_assets(vec![marked(1, &format!("url(\"{RIG}\")"))], &assets());

    let root = |declaration: &str| {
        let at = css.find(declaration).unwrap_or_else(|| {
            panic!("{declaration} missing from the emitted rule: {css}");
        });
        let value = &css[at + declaration.len()..];
        value[..value.find(';').unwrap_or(value.len())].to_string()
    };

    assert_eq!(
        root("content:url("),
        root("background-image:url("),
        "the two url-valued declarations of one generated box disagree: {css}"
    );
}

/// The regression case `3ac8e6f` closed and this defect reopened. The class is derived
/// from the rule the element will receive, so signing the captured spelling folds the rig's
/// ephemeral port into the name and renames every class on every run.
#[test]
fn the_class_does_not_change_when_only_the_rigs_origin_does() {
    let at = |origin: &str| {
        let mut style = Styles::new();
        style.insert("content".into(), format!("url(\"{origin}/mark.png\")"));
        let mut node = span(1);
        node.pseudos.insert(
            "::before".into(),
            Pseudo {
                content: format!("url(\"{origin}/mark.png\")"),
                style,
            },
        );
        (
            vec![node],
            BTreeMap::from([(format!("{origin}/mark.png"), LOCAL.to_string())]),
        )
    };

    let (nodes, map) = at("http://localhost:59898");
    let (first, css) = classes_with_assets(nodes, &map);
    let (nodes, map) = at("http://localhost:64116");
    let (second, _) = classes_with_assets(nodes, &map);

    assert_eq!(
        first[0], second[0],
        "the class name folds the capture rig's ephemeral port, so it differs on every run"
    );
    assert!(
        css.contains(LOCAL),
        "the rule was never localised, so the classes agree for the wrong reason: {css}"
    );
}

/// The neighbouring wrong shape, guarded so a fix cannot pass by deleting the declaration.
#[test]
fn keeps_the_content_declaration_it_localises() {
    let (_, css) = classes_with_assets(vec![marked(1, &format!("url(\"{RIG}\")"))], &assets());

    assert_eq!(
        css.matches("content:").count(),
        1,
        "the localised content declaration was dropped or duplicated: {css}"
    );
}

/// A URL the capture never downloaded has no entry to localise, so passthrough is correct.
/// Dropping it would lose a reference the page really made.
#[test]
fn passes_through_content_naming_an_asset_that_was_never_downloaded() {
    let missing = "url(\"http://localhost:59898/never-fetched.png\")";
    let (_, css) = classes_with_assets(vec![marked(1, missing)], &assets());

    assert!(
        css.contains(&format!("content:{missing}")),
        "an unlocalisable content url was not passed through verbatim: {css}"
    );
}

/// The inverse guard. A generated box that holds no URL anywhere must be untouched by the
/// change — same declaration, quotes intact, and the same class whether assets exist or not.
#[test]
fn leaves_a_box_that_holds_no_url_exactly_as_captured() {
    let plain_box = |ordinal| {
        let mut style = Styles::new();
        style.insert("color".into(), "red".into());
        style.insert("content".into(), "\"MARK\"".into());
        let mut node = span(ordinal);
        node.pseudos.insert(
            "::before".into(),
            Pseudo {
                content: "\"MARK\"".into(),
                style,
            },
        );
        node
    };

    let (with, css) = classes_with_assets(vec![plain_box(1)], &assets());
    let (without, _) = classes_with_assets(vec![plain_box(1)], &BTreeMap::new());

    assert!(
        css.contains("content:\"MARK\";"),
        "a plain string content was rewritten or requoted: {css}"
    );
    assert_eq!(
        with, without,
        "an asset map changed the class of an element that references no asset at all"
    );
}

/// A generated box the user agent produced declares no content of its own, and localisation
/// must not invent one.
#[test]
fn adds_no_content_declaration_to_a_box_that_carries_none() {
    let mut node = span(1);
    let mut style = Styles::new();
    style.insert("background-image".into(), format!("url(\"{RIG}\")"));
    node.pseudos.insert(
        "::backdrop".into(),
        Pseudo {
            content: String::new(),
            style,
        },
    );

    let (_, css) = classes_with_assets(vec![node], &assets());

    assert!(
        !css.contains("content:"),
        "a content declaration was fabricated for a box that had none: {css}"
    );
    assert!(
        css.contains(LOCAL),
        "the box's own style was not localised either, so this proves nothing: {css}"
    );
}
