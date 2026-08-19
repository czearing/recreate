//! Which condition decided a property, not merely that one did.
//!
//! The emitter has to put an override back under one particular prelude. Withdrawing every
//! condition at once cannot say which, so an element under two conditions would have each
//! override restated under both and the branch below one of them would paint the other's.

use super::super::{style, walk};
use super::{chains, decided};
use serde_json::{Value, json};

const MEDIA: &str = "@media (min-width: 100px)";
const CONTAINER: &str = "@container (max-width: 400px)";

/// One element, two live conditions deciding one property each, plus the identity condition a
/// capture writes around every sheet linked `media="all"`.
fn scene() -> Value {
    json!({
        "elements": [{
            "path": "/main/p",
            "classes": ["box"],
            "baked": { "color": "red", "padding-left": "9px", "font-weight": "700" }
        }],
        "matching": {
            "@media (min-width: 100px)": ["/main/p"],
            "@container (max-width: 400px)": ["/main/p"],
            "@media all": ["/main/p"]
        },
        "sheets": [[
            { "selectorText": ".box", "declarations": { "color": "blue", "padding-left": "1px" } },
            {
                "prelude": "@media (min-width: 100px)",
                "conditionText": "(min-width: 100px)",
                "rules": [style(".box", "color", "red")]
            },
            {
                "prelude": "@container (max-width: 400px)",
                "conditionText": "(max-width: 400px)",
                "rules": [style(".box", "padding-left", "9px")]
            },
            {
                "prelude": "@media all",
                "conditionText": "all",
                "rules": [style(".box", "font-weight", "700")]
            }
        ]]
    })
}

/// Each override credited to the chain that actually moved it. Pooling them would let the
/// emitter restate the media override inside the container band, painting it at a width the
/// source never did.
#[test]
fn credits_each_property_to_the_chain_that_moved_it() {
    let result = walk(scene());
    assert_eq!(decided(&result, "/main/p", MEDIA), ["color"]);
    assert_eq!(decided(&result, "/main/p", CONTAINER), ["padding-left"]);
}

/// `@media all` matches every device, so it is the identity condition and has no second arm.
/// Withdrawing against it takes a declaration out of the base rule to answer a question that
/// is never asked.
#[test]
fn credits_nothing_to_a_chain_with_no_false_branch() {
    let mut credited = chains(&walk(scene()), "/main/p");
    credited.sort();
    assert_eq!(credited, [CONTAINER, MEDIA]);
}

/// The identity condition wrapping a real one is what a sheet linked `media="all"` produces,
/// and the chain the emitter re-publishes is both layers. The key must be the whole chain, or
/// the override is credited to a prelude the output never states.
#[test]
fn keys_a_nested_chain_by_every_layer_the_emitter_re_publishes() {
    let mut scene = scene();
    scene["sheets"][0][1] = json!({
        "prelude": "@media all",
        "conditionText": "all",
        "rules": [{
            "prelude": "@media (min-width: 100px)",
            "conditionText": "(min-width: 100px)",
            "rules": [style(".box", "color", "red")]
        }]
    });

    assert_eq!(
        decided(
            &walk(scene),
            "/main/p",
            "@media all{@media (min-width: 100px)"
        ),
        ["color"]
    );
}
