use super::structural_css::class_maps;
use crate::model::{Attributes, Node, PageState, Pseudo, Rect, Styles, Viewport};
use std::collections::{BTreeMap, HashSet};

/// A decoration whose payload is also present in its captured style map, as capture records it.
pub fn pseudo(content: &str, color: &str) -> Pseudo {
    let mut style = Styles::new();
    style.insert("color".into(), color.into());
    style.insert("content".into(), content.into());
    Pseudo {
        content: content.into(),
        style,
    }
}

/// A `span` whose every other field is fixed, so a test varies only the slot under study.
pub fn span(ordinal: usize) -> Node {
    let mut style = Styles::new();
    style.insert("display".into(), "inline-block".into());
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: format!("startup>html>body:nth-of-type(1)>span:nth-of-type({ordinal})"),
        parent: Some("startup>html>body:nth-of-type(1)".into()),
        tag: "span".into(),
        text: String::new(),
        attributes: Attributes::new(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 20.0,
        },
        style,
        ..Default::default()
    }
}

/// Runs the real minter over a startup layer and returns the class of each node, in order,
/// together with the stylesheet it wrote.
pub fn classes_of(nodes: Vec<Node>) -> (Vec<String>, String) {
    classes_with_assets(nodes, &BTreeMap::new())
}

/// The same, against a page whose assets were downloaded. Localisation is what the emitted
/// rule and the class identity are both derived from, so a test of either needs this map.
pub fn classes_with_assets(
    nodes: Vec<Node>,
    assets: &BTreeMap<String, String>,
) -> (Vec<String>, String) {
    let paths: Vec<String> = nodes.iter().map(|node| node.path.clone()).collect();
    let state = PageState {
        url: "https://example.com".into(),
        title: "Example".into(),
        viewport: Viewport {
            width: 1920,
            height: 1080,
            dpr: 1.0,
        },
        startup_nodes: nodes,
        ..Default::default()
    };
    let mut css = String::new();
    let mut emitted = HashSet::new();
    let maps = class_maps(
        std::slice::from_ref(&state),
        &BTreeMap::new(),
        assets,
        &mut css,
        &mut emitted,
        None,
    );
    let classes = paths
        .iter()
        .map(|path| maps[0].get(path).cloned().unwrap_or_default())
        .collect();
    (classes, css)
}

/// The reported defect. Two elements identical in every field except which slot their
/// decoration occupies must not answer to one class, because the rule is written once per
/// class and the second element would render the first one's decoration on the wrong side.
///
/// `::before` generates the first child box and `::after` the last, so the swap moves a
/// prefix marker to the suffix position and reorders the strings feeding the accessible name.
#[test]
fn distinguishes_a_leading_decoration_from_a_trailing_one() {
    let mut lead = span(1);
    lead.pseudos
        .insert("::before".into(), pseudo("\"MARK\"", "red"));
    let mut trail = span(2);
    trail
        .pseudos
        .insert("::after".into(), pseudo("\"MARK\"", "red"));

    let (classes, css) = classes_of(vec![lead, trail]);

    assert_ne!(
        classes[0], classes[1],
        "a leading and a trailing decoration with the same payload share a class: {css}"
    );
    assert_eq!(
        css.matches("::before{").count(),
        1,
        "expected exactly one leading rule: {css}"
    );
    assert_eq!(
        css.matches("::after{").count(),
        1,
        "expected exactly one trailing rule: {css}"
    );
}

/// The concatenation is not saved by the payloads differing in length. An element carrying
/// both slots and an element carrying one slot whose payload is the two run together must
/// still be told apart, which is what an unframed encoding cannot do.
#[test]
fn distinguishes_two_decorations_from_one_carrying_both_payloads() {
    let mut both = span(1);
    both.pseudos
        .insert("::before".into(), pseudo("\"A\"", "red"));
    both.pseudos
        .insert("::after".into(), pseudo("\"B\"", "red"));
    let mut joined = span(2);
    joined
        .pseudos
        .insert("::before".into(), pseudo("\"A\"\"B\"", "red"));

    let (classes, _) = classes_of(vec![both, joined]);

    assert_ne!(
        classes[0], classes[1],
        "two decorations collided with one carrying both payloads"
    );
}

/// An absent slot must not encode as the same bytes as a present but empty one.
#[test]
fn distinguishes_no_decoration_from_an_empty_one() {
    let plain = span(1);
    let mut empty = span(2);
    empty
        .pseudos
        .insert("::before".into(), pseudo("\"\"", "red"));

    let (classes, _) = classes_of(vec![plain, empty]);

    assert_ne!(
        classes[0], classes[1],
        "an undecorated element collided with one carrying an empty decoration"
    );
}

/// The inverse guard. The fix must not trade a collision for stylesheet bloat: two elements
/// that really are alike still share one class and one rule.
#[test]
fn still_shares_one_class_and_one_rule_between_alike_elements() {
    let mut first = span(1);
    first
        .pseudos
        .insert("::before".into(), pseudo("\"MARK\"", "red"));
    let mut second = span(2);
    second
        .pseudos
        .insert("::before".into(), pseudo("\"MARK\"", "red"));

    let (classes, css) = classes_of(vec![first, second]);

    assert_eq!(
        classes[0], classes[1],
        "alike elements were given two classes"
    );
    assert_eq!(
        css.matches("::before{").count(),
        1,
        "the shared rule was written twice: {css}"
    );
}
