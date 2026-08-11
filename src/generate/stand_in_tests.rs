//! What stands where a drawing surface stood.
//!
//! The check these tests encode is a conjunction, because the cheapest way to make a
//! "pixels are missing" check pass is to stop emitting the element: the box has to survive
//! at its measured size, carrying its class, *and* the painted content has to arrive
//! reachable from it. A recreation does not re-run the page's scripts, so a surface it
//! emits as itself can never repaint — bytes attached to anything that tag ignores are the
//! original defect with a file added.

use super::tag;
use crate::generate::jsx_attrs::attributes;
use crate::model::{Node, Rect};
use std::collections::BTreeMap;

const KEY: &str = "recreate-surface:html>body>canvas:nth-of-type(1)";

fn surface(attributes: &[(&str, &str)]) -> Node {
    Node {
        disabled: false,
        path: "html>body:nth-of-type(1)>canvas:nth-of-type(1)".into(),
        parent: Some("html>body:nth-of-type(1)".into()),
        tag: "canvas".into(),
        text: String::new(),
        attributes: attributes
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 120.0,
        },
        style: Default::default(),
        before: None,
        after: None,
    }
}

fn localised() -> BTreeMap<String, String> {
    BTreeMap::from([(KEY.to_string(), "/assets/painted.png".to_string())])
}

/// The subject. The element that stood over the painted box is emitted as one that paints
/// a source, and the box's own attributes — which the substitution does not move — come
/// with it.
#[test]
fn emits_the_element_that_paints_what_the_capture_read() {
    let node = surface(&[
        (crate::surface_content::ATTRIBUTE, KEY),
        ("width", "200"),
        ("height", "120"),
        ("id", "painted"),
    ]);
    let assets = localised();
    assert_eq!(tag(&node, &assets), "img");
    assert_eq!(
        attributes(&node, &assets),
        " height={\"120\"} id={\"painted\"} width={\"200\"} src={\"/assets/painted.png\"} alt={\"\"}"
    );
}

/// The key is an internal channel between the two stages that share its name. Emitting it
/// would put an artifact in the output naming bytes by a scheme nothing can fetch.
#[test]
fn never_emits_the_key_it_carried_the_content_by() {
    let node = surface(&[(crate::surface_content::ATTRIBUTE, KEY), ("width", "200")]);
    for assets in [localised(), BTreeMap::new()] {
        let output = attributes(&node, &assets);
        assert!(
            !output.contains(crate::surface_content::ATTRIBUTE),
            "{output}"
        );
        assert!(!output.contains("recreate-surface:"), "{output}");
    }
}

/// A surface whose bytes never reached the project is emitted as itself, at its measured
/// size. A stand-in pointing at a file that is not there is worse than the empty box it
/// replaced: it turns content that is merely missing into a broken reference.
#[test]
fn emits_the_element_itself_when_the_content_did_not_arrive() {
    let node = surface(&[(crate::surface_content::ATTRIBUTE, KEY), ("width", "200")]);
    let assets = BTreeMap::new();
    assert_eq!(tag(&node, &assets), "canvas");
    assert_eq!(attributes(&node, &assets), " width={\"200\"}");
}

/// An element that painted nothing must gain nothing. Every element in the page reaches
/// this call, so the substitution has to be keyed on content that was actually read.
#[test]
fn leaves_an_element_with_no_painted_content_alone() {
    let mut node = surface(&[("width", "200")]);
    node.tag = "div".into();
    let assets = localised();
    assert_eq!(tag(&node, &assets), "div");
    assert_eq!(attributes(&node, &assets), " width={\"200\"}");
}

/// The name has to change channel with the content. `<img>` exposes `alt`, and an image
/// carrying both `alt` and `aria-label` announces the label while reading as decorative,
/// so the label is translated rather than copied — the same rule the relocated-graphic
/// stand-in applies, from the same place.
#[test]
fn translates_the_name_the_replaced_element_exposed() {
    let node = surface(&[
        (crate::surface_content::ATTRIBUTE, KEY),
        ("aria-label", "sales by quarter"),
        ("aria-describedby", "legend"),
    ]);
    let output = attributes(&node, &localised());
    assert_eq!(
        output,
        " aria-describedby={\"legend\"} src={\"/assets/painted.png\"} alt={\"sales by quarter\"}"
    );
}
