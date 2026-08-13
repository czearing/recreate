use super::BeforeChange;
use crate::generate::animations::append;
use crate::model::{Animation, Node, Rect};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

/// The panel the scene authors: a resting `translate` of zero, and an entry transition whose
/// only record of where it starts is the authored `@starting-style` rule.
fn panel() -> Node {
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        path: "html>body:nth-of-type(1)>div:nth-of-type(1)".into(),
        parent: Some("html>body:nth-of-type(1)".into()),
        tag: "div".into(),
        text: String::new(),
        attributes: [("class".to_string(), "panel".to_string())]
            .into_iter()
            .collect(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 69.0,
        },
        style: [
            ("opacity".to_string(), "1".to_string()),
            ("translate".to_string(), "0px".to_string()),
        ]
        .into_iter()
        .collect(),
        before: None,
        after: None,
        disabled: false,
        rtl: false,
        ..Default::default()
    }
}

/// What the browser reports for that entry transition.
///
/// `opacity` arrives correct because the property has no keyword form. `translate` arrives
/// as `none` — its initial value — which is indistinguishable from an authored identity, so
/// nothing downstream can tell the start was lost.
fn entry_animation() -> Animation {
    Animation {
        target: panel().path,
        name: String::new(),
        keyframes: vec![
            json!({"computedOffset":0.0,"easing":"linear","opacity":"0","translate":"none","clipPath":"none"}),
            json!({"computedOffset":1.0,"easing":"linear","opacity":"1","translate":"0px","clipPath":"inset(0px)"}),
        ],
        timing: json!({
            "duration": 400,
            "iterations": 1,
            "direction": "normal",
            "fill": "backwards",
            "playState": "running",
            "playbackRate": 1
        }),
    }
}

fn authored_rules() -> Vec<String> {
    vec![
        ".panel { opacity: 1; translate: 0 0; transition: opacity 0.4s linear, translate 0.4s linear; }".into(),
        "@starting-style{.panel { opacity: 0; translate: 0px 24px; clip-path: inset(40px); }}".into(),
    ]
}

fn emit(nodes: &[Node], rules: &[String], animations: &[Animation]) -> String {
    let mut classes = BTreeMap::from([(panel().path, "base".to_string())]);
    let mut css = String::new();
    append(
        animations,
        &BTreeSet::new(),
        &BeforeChange::new(rules, nodes),
        &mut classes,
        &mut css,
    );
    css
}

/// The defect. The emitted opening frame must carry the distance the author wrote, not the
/// keyword the animation API substituted for it.
#[test]
fn seeds_the_opening_frame_from_the_authored_before_change_style() {
    let css = emit(&[panel()], &authored_rules(), &[entry_animation()]);
    assert!(
        css.contains("translate:0px 24px"),
        "opening frame lost the authored start distance: {css}"
    );
    assert!(
        !css.contains("translate:none"),
        "opening frame still carries the initial keyword: {css}"
    );
}

/// A keyframe names its property by the camelCase IDL name while the authored rule uses the
/// CSS name, so a repair that matched the two literally would reach only the properties
/// whose two spellings happen to coincide.
#[test]
fn matches_a_camel_cased_keyframe_property_to_its_authored_declaration() {
    let css = emit(&[panel()], &authored_rules(), &[entry_animation()]);
    assert!(
        css.contains("clip-path:inset(40px)"),
        "camel-cased property was not matched to its authored declaration: {css}"
    );
}

/// The closing frame is the resting style the capture measured, and no authored
/// before-change value may reach it — seeding both ends would erase the movement entirely.
#[test]
fn leaves_the_closing_frame_at_the_measured_resting_value() {
    let css = emit(&[panel()], &authored_rules(), &[entry_animation()]);
    let closing = css
        .split("100%{")
        .nth(1)
        .expect("no closing frame emitted")
        .split('}')
        .next()
        .unwrap()
        .to_string();
    assert!(
        closing.contains("translate:0px;"),
        "closing frame was overwritten: {closing}"
    );
    assert!(
        !closing.contains("24px"),
        "authored start value leaked into the closing frame: {closing}"
    );
}

/// A property the API reports correctly must be left exactly as measured. The authored rule
/// declares `opacity: 0` too, so a repair that simply overlays every authored declaration
/// would pass the test above while silently taking authority away from the capture.
#[test]
fn defers_to_the_capture_for_a_property_the_animation_api_reports() {
    let mut animation = entry_animation();
    animation.keyframes[0] = json!({"computedOffset":0.0,"opacity":"0.25","translate":"none"});
    let css = emit(&[panel()], &authored_rules(), &[animation]);
    assert!(
        css.contains("opacity:0.25"),
        "measured opening opacity was overwritten by the authored rule: {css}"
    );
}

/// A node the authored selector does not reach keeps its recorded frames. Without this the
/// repair would apply one element's entry distance to every animated element on the page.
#[test]
fn leaves_an_element_the_authored_selector_does_not_reach_untouched() {
    let mut other = panel();
    other.path = "html>body:nth-of-type(1)>div:nth-of-type(2)".into();
    other.attributes.insert("class".into(), "sidebar".into());
    let mut animation = entry_animation();
    animation.target.clone_from(&other.path);
    let mut classes = BTreeMap::from([(other.path.clone(), "base".to_string())]);
    let mut css = String::new();
    append(
        &[animation],
        &BTreeSet::new(),
        &BeforeChange::new(&authored_rules(), &[other]),
        &mut classes,
        &mut css,
    );
    assert!(
        css.contains("translate:none"),
        "an unmatched element was seeded from another element's rule: {css}"
    );
}

/// The declarations of a `@starting-style` rule never reach an element's resting style, so
/// they must not be admitted to the authored bake that produces it.
#[test]
fn keeps_the_before_change_declarations_out_of_the_resting_style() {
    let mut styles = panel().style;
    crate::generate::authored_css::normalize(&mut styles, &panel(), &authored_rules());
    assert_eq!(
        styles.get("translate").map(String::as_str),
        Some("0px"),
        "the entry start value was baked as a resting value: {styles:?}"
    );
}
