//! A property that returns to its default inside a viewport band, or inside an
//! interaction state, must be said out loud. Silence in a cascade is not neutral: the
//! base declaration keeps winning, so the artifact asserts the value the source withdrew.
use crate::model::{Node, Pseudo, Rect, Viewport};

pub(super) fn box_node(path: &str, style: &[(&str, &str)]) -> Node {
    Node {
        writing_mode: Default::default(),
        disabled: false,
        rtl: false,
        path: path.into(),
        parent: Some("html>body".into()),
        tag: "div".into(),
        text: "Box".into(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 240.0,
            height: 120.0,
        },
        style: style
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
        before: None,
        after: None,
    }
}

fn viewports() -> (Viewport, Viewport) {
    (
        Viewport {
            width: 1920,
            height: 1080,
            dpr: 1.0,
        },
        Viewport {
            width: 390,
            height: 844,
            dpr: 1.0,
        },
    )
}

pub(super) fn band_rule(base: &Node, narrow: &Node) -> String {
    band_rule_with(base, narrow, &[])
}

pub(super) fn band_rule_with(base: &Node, narrow: &Node, rules: &[String]) -> String {
    let (wide, small) = viewports();
    super::responsive::append_node_rules(
        base,
        narrow,
        None,
        (&wide, &small),
        "band",
        &Default::default(),
        rules,
        false,
        false,
    )
}

/// The filed defect. `border-bottom-width` descends into the element's baseline as the
/// viewport narrows, so the capture prunes it and the band falls silent about it.
#[test]
fn says_that_a_property_went_back_to_its_default_inside_the_band() {
    let css = band_rule(&reverting_base(), &reverting_narrow());
    assert!(
        css.contains("border-bottom-width:"),
        "the band must declare the reverted property instead of letting the base 13px \
         keep winning: {css}"
    );
}

/// Neutralising the base declaration by repeating it is not a repair: the band would
/// still paint 13px, which is exactly the value the source withdrew.
#[test]
fn never_restates_the_base_value_it_is_overriding() {
    let css = band_rule(&reverting_base(), &reverting_narrow());
    assert!(!css.contains("border-bottom-width:13px"), "{css}");
}

/// A reversion must not be widened into a blanket reset of the element, which would
/// discard every other declaration the band is in force alongside.
#[test]
fn refuses_the_reverted_key_only_never_the_whole_rule() {
    let css = band_rule(&reverting_base(), &reverting_narrow());
    assert!(!css.contains("all:"), "{css}");
}

fn reverting_base() -> Node {
    box_node(
        "html>body>div",
        &[
            ("border-bottom-style", "solid"),
            ("border-bottom-width", "13px"),
        ],
    )
}

fn reverting_narrow() -> Node {
    box_node("html>body>div", &[("border-bottom-style", "solid")])
}

/// The mirrored half. A property that ascends out of the baseline was never lost, so the
/// fix must not be credited for it, and it must not change.
#[test]
fn still_emits_a_property_that_appeared_in_the_band() {
    let base = box_node("html>body>div", &[("border-bottom-style", "solid")]);
    let narrow = box_node(
        "html>body>div",
        &[
            ("border-bottom-style", "solid"),
            ("border-bottom-width", "2px"),
        ],
    );
    let css = band_rule(&base, &narrow);
    assert!(css.contains("border-bottom-width:2px"), "{css}");
}

/// The difference must stay a difference. A key whose value is unchanged still emits
/// nothing, or every band restates every property and real changes drown in the noise.
#[test]
fn stays_silent_about_a_property_that_did_not_move() {
    let styles: &[(&str, &str)] = &[
        ("border-bottom-style", "solid"),
        ("border-bottom-width", "13px"),
        ("color", "rgb(0, 0, 0)"),
    ];
    let base = box_node("html>body>div", styles);
    let narrow = box_node("html>body>div", styles);
    assert_eq!(band_rule(&base, &narrow), "");
}

/// Reverting one property must not drag the others out with it: per-key refusal, not a
/// whole-rule reset.
#[test]
fn reverts_only_the_key_that_went_back() {
    let base = box_node(
        "html>body>div",
        &[
            ("border-bottom-style", "solid"),
            ("border-bottom-width", "13px"),
            ("background-color", "rgb(255, 0, 0)"),
        ],
    );
    let narrow = box_node(
        "html>body>div",
        &[
            ("border-bottom-style", "solid"),
            ("background-color", "rgb(0, 0, 255)"),
        ],
    );
    let css = band_rule(&base, &narrow);
    assert!(css.contains("border-bottom-width:revert"), "{css}");
    assert!(css.contains("background-color:rgb(0, 0, 255)"), "{css}");
    assert!(!css.contains("border-bottom-style"), "{css}");
}

fn pseudo(content: &str, style: &[(&str, &str)]) -> Pseudo {
    Pseudo {
        content: content.into(),
        style: style
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
    }
}

/// The pseudo-element branch runs the same difference, so it inherits the same repair.
#[test]
fn says_that_a_pseudo_element_property_went_back_to_its_default() {
    let mut base = box_node("html>body>div", &[]);
    base.before = Some(pseudo("\"x\"", &[("color", "rgb(255, 0, 0)")]));
    let mut narrow = box_node("html>body>div", &[]);
    narrow.before = Some(pseudo("\"x\"", &[]));
    let css = band_rule(&base, &narrow);
    assert!(css.contains(".band::before{color:revert"), "{css}");
}

#[cfg(test)]
#[path = "inert_reset_tests.rs"]
mod inert;
