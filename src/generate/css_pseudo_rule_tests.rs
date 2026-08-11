//! What a generated box actually writes into the stylesheet, as distinct from what it is named.
//!
//! Identity and output are two halves of one rule, so they are pinned separately: a fix that
//! makes the names injective by emitting every slot for every class would satisfy the counts
//! while fabricating decoration, and these are the tests that refuse it.

use super::css_pseudo_identity_tests::{classes_of, pseudo, span};
use crate::model::{PageState, Specification, Viewport};
use std::collections::BTreeMap;

/// The trap that would satisfy the slot counts while fabricating decoration: emitting both
/// slots for every class. An element that declared one generated box receives exactly one.
#[test]
fn writes_no_rule_for_a_slot_the_element_never_used() {
    let mut lead = span(1);
    lead.before = Some(pseudo("\"MARK\"", "red"));

    let (_, css) = classes_of(vec![lead]);

    assert_eq!(css.matches("::before{").count(), 1, "{css}");
    assert_eq!(
        css.matches("::after{").count(),
        0,
        "a trailing rule was fabricated for an element that has no trailing decoration: {css}"
    );
}

/// Two elements alike enough to share a class share one rule, and the boxes they generate are
/// part of that rule. Writing the decoration outside the emit-once guard repeated it verbatim
/// once per element, which is stylesheet bloat in exactly the case the class exists to collapse.
#[test]
fn writes_a_shared_decoration_once_across_the_elements_that_share_it() {
    let decorated = |ordinal: usize| {
        let mut node = span(ordinal);
        node.path = format!("html>body:nth-of-type(1)>span:nth-of-type({ordinal})");
        node.parent = Some("html>body:nth-of-type(1)".into());
        node.before = Some(pseudo("\"MARK\"", "red"));
        node
    };
    let specification = Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![PageState {
            viewport: Viewport {
                width: 1920,
                height: 1080,
                dpr: 1.0,
            },
            nodes: vec![decorated(1), decorated(2)],
            ..Default::default()
        }],
        interactions: Vec::new(),
        transitions: Vec::new(),
    };
    let assets = BTreeMap::new();
    let output = super::css_base::build(super::css_base::Request {
        specification: &specification,
        assets: &assets,
        prefix: "r",
        include_interactions: false,
        reuse: None,
        cache: None,
        path_override: None,
        timing: &|_: &str| {},
    });

    assert_eq!(
        output.css.matches("::before{").count(),
        1,
        "one decoration was written once per element that shares it: {}",
        output.css
    );
}

/// The captured style map already carries `content`, and both emitters also wrote it from
/// `Pseudo::content`, so every generated rule declared it twice.
#[test]
fn declares_the_generated_content_once() {
    let mut lead = span(1);
    lead.before = Some(pseudo("\"MARK\"", "red"));

    let (_, css) = classes_of(vec![lead]);

    assert_eq!(
        css.matches("content:").count(),
        1,
        "the content declaration was duplicated: {css}"
    );
}
