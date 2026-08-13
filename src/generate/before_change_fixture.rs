//! The entry-transition scene both before-change test files assert against.
//!
//! One element, one authored `@starting-style` block and one animation record whose opening
//! frame the browser reported lossily. Shared so that the module owning *which value wins*
//! and the module owning *which authoring form is read* assert against the same scene, and
//! a change to one cannot silently stop exercising the other.

use crate::generate::animations::append;
use crate::generate::before_change::BeforeChange;
use crate::model::{Animation, Node, Rect};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

/// The panel the scene authors: a resting `translate` of zero, and an entry transition whose
/// only record of where it starts is the authored `@starting-style` rule.
pub(super) fn panel() -> Node {
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
pub(super) fn entry_animation() -> Animation {
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

/// The scene's stylesheet in the top-level authoring form.
pub(super) fn authored_rules() -> Vec<String> {
    vec![
        ".panel { opacity: 1; translate: 0 0; transition: opacity 0.4s linear, translate 0.4s linear; }".into(),
        "@starting-style{.panel { opacity: 0; translate: 0px 24px; clip-path: inset(40px); }}".into(),
    ]
}

/// The same stylesheet in the nested authoring form, in the shape the capture records it:
/// the block sits inside the style rule, its declarations are bare, and the enclosing
/// prelude is the only selector that reaches them.
pub(super) fn nested_authored_rules() -> Vec<String> {
    vec![
        concat!(
            ".panel {\n  opacity: 1; translate: 0px;",
            " transition: opacity 0.4s linear, translate 0.4s linear;\n",
            "  @starting-style {\n  opacity: 0; translate: 0px 24px; clip-path: inset(40px);\n}\n}"
        )
        .into(),
    ]
}

pub(super) fn emit(nodes: &[Node], rules: &[String], animations: &[Animation]) -> String {
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
