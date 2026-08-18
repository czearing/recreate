//! Entry motion must be a property of the page, not of how fast the capture ran.
//!
//! A transition that runs on first render is over in a few hundred milliseconds, so a record of
//! it only ever existed when the capture happened to read the page mid-flight. Reading it out of
//! the authored `@starting-style` and the element's own `transition` longhands instead says the
//! same thing on every run, on every machine.

use crate::model::{Animation, Node, Styles};
use std::collections::HashMap;

fn node(style: &[(&str, &str)]) -> Node {
    let mut node = Node {
        path: "html>body:nth-of-type(1)>div:nth-of-type(1)".into(),
        tag: "div".into(),
        ..Node::default()
    };
    node.style = style
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    node
}

fn before(declared: &[(&str, &str)]) -> HashMap<String, Styles> {
    let mut map = HashMap::new();
    map.insert(
        "html>body:nth-of-type(1)>div:nth-of-type(1)".to_string(),
        declared
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect::<Styles>(),
    );
    map
}

fn emitted(built: &[Animation]) -> String {
    let mut css = String::new();
    for (index, animation) in built.iter().enumerate() {
        crate::generate::animation_keyframes::append(animation, &format!("entry{index}"), &mut css);
    }
    css
}

/// The whole defect: the page states the motion statically, so the record must not depend on a
/// reading being in flight when the capture arrived.
#[test]
fn a_started_property_the_element_transitions_becomes_entry_motion() {
    let built = super::animations(
        &before(&[("opacity", "0")]),
        &[node(&[
            ("transition-property", "opacity"),
            ("transition-duration", "0.12s"),
            ("transition-timing-function", "linear"),
        ])],
        &[],
    );
    assert_eq!(built.len(), 1);
    assert_eq!(built[0].timing["duration"], 120.0);
    assert_eq!(built[0].timing["fill"], "backwards");
    assert_eq!(
        emitted(&built),
        "@keyframes entry0{0%{animation-timing-function:linear;opacity:0;}}\n"
    );
}

/// Only the opening frame, because the value it travels to is the element's own resting value —
/// which the recreation already carries, and which cannot be named at all when it equals the
/// property's initial value, since a value equal to the baseline is never recorded.
#[test]
fn only_the_opening_frame_is_stated() {
    let built = super::animations(
        &before(&[("opacity", "0")]),
        &[node(&[
            ("transition-property", "opacity"),
            ("transition-duration", "120ms"),
        ])],
        &[],
    );
    assert_eq!(built[0].keyframes.len(), 1);
}

/// A started value the element does not transition never moves, so describing it as motion would
/// invent an animation the page does not run.
#[test]
fn a_started_property_with_no_transition_is_not_motion() {
    let built = super::animations(
        &before(&[("translate", "0 24px")]),
        &[node(&[
            ("transition-property", "opacity"),
            ("transition-duration", "0.12s"),
        ])],
        &[],
    );
    assert!(built.is_empty());
}

/// A zero-length transition delivers its value instantly, which is a resting value and not a
/// motion. Emitting it would publish a keyframes block that never plays.
#[test]
fn an_instant_transition_is_not_motion() {
    let built = super::animations(
        &before(&[("opacity", "0")]),
        &[node(&[
            ("transition-property", "opacity"),
            ("transition-duration", "0s"),
        ])],
        &[],
    );
    assert!(built.is_empty());
}

/// The longhands are parallel lists that repeat, so the index a property sits at decides its
/// duration, delay and easing — including through a function whose own arguments are commas.
#[test]
fn each_property_takes_the_timing_at_its_own_index() {
    let built = super::animations(
        &before(&[("opacity", "0"), ("translate", "0 24px")]),
        &[node(&[
            ("transition-property", "opacity, translate"),
            ("transition-duration", "0.4s, 90ms"),
            ("transition-delay", "10ms, 20ms"),
            (
                "transition-timing-function",
                "linear, cubic-bezier(0.4, 0, 0.2, 1)",
            ),
        ])],
        &[],
    );
    let opacity = built
        .iter()
        .find(|a| a.keyframes[0]["opacity"] == "0")
        .unwrap();
    let translate = built
        .iter()
        .find(|a| a.keyframes[0]["translate"] == "0 24px")
        .unwrap();
    assert_eq!(opacity.timing["duration"], 400.0);
    assert_eq!(opacity.timing["delay"], 10.0);
    assert_eq!(translate.timing["duration"], 90.0);
    assert_eq!(translate.timing["easing"], "cubic-bezier(0.4, 0, 0.2, 1)");
}

/// `all` names every property at once, which is the common authoring and the one a list-based
/// reading silently misses.
#[test]
fn a_transition_on_all_covers_every_started_property() {
    let built = super::animations(
        &before(&[("opacity", "0")]),
        &[node(&[
            ("transition-property", "all"),
            ("transition-duration", "0.2s"),
        ])],
        &[],
    );
    assert_eq!(built.len(), 1);
}

/// Where the capture did catch the motion it measured what actually ran, so it stays the
/// authority and this stage must not publish a second animation over it.
#[test]
fn a_property_the_capture_already_recorded_is_left_alone() {
    let recorded = Animation {
        target: "html>body:nth-of-type(1)>div:nth-of-type(1)".into(),
        name: String::new(),
        keyframes: vec![
            serde_json::json!({"opacity": "0"}),
            serde_json::json!({"opacity": "1"}),
        ],
        timing: serde_json::json!({"duration": 120}),
    };
    let built = super::animations(
        &before(&[("opacity", "0")]),
        &[node(&[
            ("transition-property", "opacity"),
            ("transition-duration", "0.12s"),
        ])],
        &[recorded],
    );
    assert!(built.is_empty());
}

/// The synthesized record reaches the stylesheet through the same stage every recorded one does,
/// so a guard that only ever saw two-frame records cannot silently drop it.
#[test]
fn the_entry_motion_reaches_the_emitted_stylesheet() {
    let built = super::animations(
        &before(&[("opacity", "0")]),
        &[node(&[
            ("transition-property", "opacity"),
            ("transition-duration", "0.12s"),
        ])],
        &[],
    );
    let mut classes = std::collections::BTreeMap::new();
    classes.insert(built[0].target.clone(), "r0".to_string());
    let mut css = String::new();
    crate::generate::animations::append(
        &built,
        &std::collections::BTreeSet::new(),
        &crate::generate::before_change::BeforeChange::default(),
        &mut classes,
        &mut css,
    );
    assert!(css.contains("opacity:0;"), "{css}");
    assert!(css.contains("@keyframes recreate"), "{css}");
    assert!(
        classes.values().any(|value| value.contains(" a")),
        "{classes:?}"
    );
}

/// The synthesized motion has to travel the same path the seeded records do, or it exists only
/// in this module and never reaches a stylesheet.
#[test]
fn seeding_a_state_carries_the_entry_motion_with_it() {
    let rules = vec!["@starting-style { div { opacity: 0 } }".to_string()];
    let nodes = vec![node(&[
        ("transition-property", "opacity"),
        ("transition-duration", "0.12s"),
    ])];
    let before = crate::generate::before_change::BeforeChange::new(&rules, &nodes)
        .with_entry_motion(&nodes, &[]);
    let seeded = before.seed(&[]);
    assert_eq!(seeded.len(), 1);
    assert_eq!(seeded[0].keyframes[0]["opacity"], "0");
}
