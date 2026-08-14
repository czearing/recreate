//! A generated box's `content` is a declaration the generator emits from its own field, and
//! the generator localises it by looking the URL up in what this walk collected. So the walk
//! has to enumerate that field, not merely the style map beside it.
//!
//! Until it did, the two agreed only because `generated_boxes.js` writes the value into both
//! places and a `url()` always differs from a generated box's `none` baseline. That is a
//! pruning coincidence: the moment the reduction drops `content` from the map, the generator
//! is localising a value nothing ever downloaded, and the failure is the silent one — no
//! error, no blocker, exit 0, and an emitted rule pointing at an origin that is gone.

use super::reach_tests::{assets, url};
use serde_json::json;

/// A `content` url is the only reference a generated box makes. This is the shape the
/// baseline reduction produces for a state whose box matched its own baseline.
#[test]
fn collects_an_asset_named_only_by_a_generated_boxs_content() {
    assert_eq!(
        assets(json!({
            "tag": "div",
            "pseudos": { "::before": { "content": "url(\"mark.png\")", "style": {} } }
        })),
        vec![url("mark.png")]
    );
}

/// The style map beside it must keep working, and a box that names two different assets
/// must yield both — the field is an addition to the walk, not a replacement for it.
#[test]
fn collects_both_the_content_and_the_style_of_one_generated_box() {
    assert_eq!(
        assets(json!({
            "tag": "div",
            "pseudos": { "::before": {
                "content": "url(\"mark.png\")",
                "style": { "background-image": "url(\"plate.png\")" }
            } }
        })),
        vec![url("mark.png"), url("plate.png")]
    );
}

/// The value the capture actually produces today, present in both places. Collecting it
/// twice would be harmless but would prove the walk is not deduplicating; this pins that
/// one asset stays one entry.
#[test]
fn collects_a_duplicated_content_value_exactly_once() {
    assert_eq!(
        assets(json!({
            "tag": "div",
            "pseudos": { "::before": {
                "content": "url(\"mark.png\")",
                "style": { "content": "url(\"mark.png\")" }
            } }
        })),
        vec![url("mark.png")]
    );
}

/// The inverse guard. A generated box whose content is a plain string names no asset, and
/// a walk that treats the field as a URL source must not invent one from it.
#[test]
fn collects_nothing_from_a_generated_box_whose_content_is_a_string() {
    assert!(
        assets(json!({
            "tag": "div",
            "pseudos": { "::before": { "content": "\"MARK\"", "style": {} } }
        }))
        .is_empty()
    );
}

/// An element with no generated boxes at all must be unaffected, so the change cannot move
/// output on the pages that carry no decoration.
#[test]
fn collects_nothing_extra_from_an_element_with_no_generated_boxes() {
    assert_eq!(
        assets(json!({ "tag": "div", "style": { "background-image": "url(\"plate.png\")" } })),
        vec![url("plate.png")]
    );
}
