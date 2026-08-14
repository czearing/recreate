//! What the capture collects must be reachable from wherever the capture walked.
//!
//! A selector is two questions in one syntax — which nodes match, and which tree is
//! searched — and only the first was ever stated. The gate below it answers membership
//! rigorously and cannot answer scope at all, so a `document.querySelectorAll` beside a
//! walk that enters shadow roots agreed on every element in the light tree and disagreed
//! on every element outside it. The failure that produced was worse than an omission: the
//! walk still resolved the reference against the page base, so the artifact shipped a live
//! address on the capture rig's ephemeral port — dead, and different on every run.
//!
//! These tests run the shipped script under Node against a scripted DOM whose only two
//! properties are the platform's: `matches` answers about an element wherever it lives,
//! and a document-rooted query cannot cross a shadow boundary.

use serde_json::{Value, json};

use super::reach_harness::{ORIGIN, walk};

const BASE: &str = super::reach_harness::LOCATION;

/// Walks `tree` exactly as the capture does, then reports what was collected and what was
/// recorded onto each visited element.
fn capture(tree: Value, css_rules: Value) -> (Vec<String>, Vec<Value>) {
    let result = walk(&tree, &css_rules, BASE);
    (
        super::reach_harness::assets(&result),
        result["recorded"].as_array().expect("recorded").clone(),
    )
}

pub(super) fn assets(tree: Value) -> Vec<String> {
    capture(tree, json!([])).0
}

fn image(source: &str) -> Value {
    json!({ "tag": "img", "assetBearing": true, "attributes": { "src": source } })
}

fn host(shadow: Value, children: Value) -> Value {
    json!({ "tag": "div", "shadow": shadow, "children": children })
}

pub(super) fn url(path: &str) -> String {
    format!("{ORIGIN}{path}")
}

/// The subject. An `<img>` the walk reached only through a host's `shadowRoot` is an
/// element the capture recorded, so its bytes are as necessary to the artifact as any
/// other element's. Nothing about it is unusual except which tree it lives in.
#[test]
fn collects_an_image_the_walk_reached_through_a_shadow_root() {
    assert_eq!(
        assets(host(json!([image("subject.png")]), json!([]))),
        vec![url("subject.png")]
    );
}

/// One level of shadow is not the rule; the rule is the walk's reach. A host inside a
/// shadow root is what separates a fix that iterates the traversal from a fix that
/// remembered to look one place further.
#[test]
fn collects_an_image_inside_a_nested_shadow_root() {
    assert_eq!(
        assets(host(
            json!([host(json!([image("deep.png")]), json!([]))]),
            json!([])
        )),
        vec![url("deep.png")]
    );
}

/// The guard against fixing this by narrowing. An element in the document tree was always
/// collected and must stay collected, or the repair costs more than the defect.
#[test]
fn still_collects_an_image_in_the_document_tree() {
    assert_eq!(
        assets(json!({ "tag": "div", "children": [image("light.png")] })),
        vec![url("light.png")]
    );
}

/// Slotting does not move a node: a slotted element is a child of the host in the light
/// tree, so both reaches always found it. It is collected once, not twice — proof the fix
/// replaced a traversal rather than adding one beside it.
#[test]
fn collects_a_slotted_image_exactly_once() {
    let assets = assets(host(
        json!([{ "tag": "slot" }]),
        json!([image("slotted.png")]),
    ));
    assert_eq!(assets, vec![url("slotted.png")]);
}

/// The candidate-attribute path through the same loop. `srcset` names a whole set and
/// `poster` names a frame the element paints before any media loads; both are reached by
/// the same walk and neither may depend on which tree the element sits in.
#[test]
fn collects_every_candidate_a_shadow_element_advertises() {
    let assets = assets(host(
        json!([
            { "tag": "source", "assetBearing": true,
              "attributes": { "srcset": "one.png 1x, two.png 2x" } },
            { "tag": "video", "assetBearing": true, "attributes": { "poster": "still.png" } }
        ]),
        json!([]),
    ));
    assert_eq!(
        assets,
        vec![url("one.png"), url("still.png"), url("two.png")]
    );
}

/// The control from the reproduction scene, and the reason the defect was provable at all:
/// a `url()` in a shadow element's own computed style reaches the collector through the
/// node list the walk built, so it was always collected. Same function, same subtree, and
/// before the fix the opposite answer to the `<img>` beside it.
#[test]
fn collects_a_background_image_from_a_shadow_element_style() {
    let assets = assets(host(
        json!([{ "tag": "div", "style": { "background-image": "url(control.png)" } }]),
        json!([]),
    ));
    assert_eq!(assets, vec![url("control.png")]);
}

/// The invariant the defect violated, stated over a tree that exercises every reach at
/// once: a URL the capture rewrote to the page origin is a URL the capture promised the
/// artifact would contain. Recording without collecting emits a live address on the rig's
/// ephemeral port; collecting without recording downloads bytes nothing references. The
/// two halves are one decision, and this is the assertion that keeps them one.
#[test]
fn collects_every_url_it_rewrote_onto_a_walked_element() {
    let (assets, recorded) = capture(
        host(
            json!([
                image("shadow.png"),
                host(json!([image("nested.png")]), json!([])),
                { "tag": "div", "style": { "background-image": "url(shadow-bg.png)" } }
            ]),
            json!([image("light.png")]),
        ),
        json!(["body { background: url(sheet.png) }"]),
    );
    let rewritten: Vec<String> = recorded
        .iter()
        .flat_map(|attributes| attributes.as_object().expect("attributes").values())
        .filter_map(Value::as_str)
        .filter(|value| value.starts_with(ORIGIN))
        .map(str::to_string)
        .collect();
    assert_eq!(
        rewritten.len(),
        3,
        "the walk must have rewritten every image attribute it reached, or the tree \
         proves nothing"
    );
    for path in ["shadow-bg.png", "sheet.png"] {
        assert!(
            assets.contains(&url(path)),
            "the style and stylesheet reaches must still contribute, or the tree exercises \
             only one path: {path}"
        );
    }
    for value in rewritten {
        assert!(
            assets.contains(&value),
            "a reference was repointed at the capture origin but never collected, so the \
             artifact advertises a rig address that dies with the capture: {value}"
        );
    }
}

/// The complement. Widening reach must not widen membership: a reference that loads
/// another document is outside the gate in every tree, and its authored absolute URL is
/// left alone rather than swept in by a walk that now visits more elements.
#[test]
fn leaves_a_document_reference_uncollected_in_every_tree() {
    let link = json!({ "tag": "a", "attributes": { "href": "http://rig.test:59700/next.html" } });
    assert!(
        assets(host(json!([link.clone()]), json!([link]))).is_empty(),
        "a document reference reached the asset set once the walk's reach was adopted"
    );
}
