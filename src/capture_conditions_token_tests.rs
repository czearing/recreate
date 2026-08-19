//! A conditional block decides properties it never names.
//!
//! `var()` substitutes at computed-value time, so a block declaring only a custom property
//! moves whatever longhands read that token — on this element or on any descendant the token
//! inherits to — and none of those longhands is spelled anywhere in the block. A pass that
//! took either the blocks to withdraw or the properties to watch from what a block says
//! therefore has nothing to withdraw and nothing to watch, and reports the element as
//! unconditional.

use super::super::{style, walk};
use super::{chains, decided, record};
use serde_json::{Value, json};

const CONTAINER: &str = "@container (max-width: 300px)";

/// One condition, one property, three carriers: the longhand itself, a token read on the same
/// element, and a token read on a descendant. The condition holds, so the base arm can only
/// reach the output through the stage under test.
fn scene() -> Value {
    json!({
        "elements": [
            { "path": "/main", "classes": ["frame"], "baked": {} },
            { "path": "/main/a", "classes": ["arm-a"], "baked": { "padding-left": "61px" } },
            { "path": "/main/b", "classes": ["arm-b"], "baked": { "padding-left": "62px" } },
            { "path": "/main/d", "classes": ["arm-d"], "baked": { "padding-left": "63px" } }
        ],
        "matching": {
            "@container (max-width: 300px)": ["/main", "/main/a", "/main/b", "/main/d"]
        },
        "sheets": [[
            style(".arm-a", "padding-left", "4px"),
            { "selectorText": ".arm-b", "declarations": {
                "--pad-b": "5px", "padding-left": "var(--pad-b)" } },
            { "selectorText": ".frame", "declarations": { "--pad-d": "6px" } },
            style(".arm-d", "padding-left", "var(--pad-d)"),
            {
                "prelude": "@container (max-width: 300px)",
                "conditionText": "(max-width: 300px)",
                "rules": [
                    style(".arm-a", "padding-left", "61px"),
                    { "selectorText": ".arm-b", "declarations": { "--pad-b": "62px" } },
                    { "selectorText": ".frame", "declarations": {
                        "--pad-d": "63px", "letter-spacing": "3px" } }
                ]
            }
        ]]
    })
}

/// The population an earlier repair already covered, kept as the control: if this stops
/// holding, the change below traded one carrier for another rather than adding one.
#[test]
fn still_names_a_property_the_conditional_block_declares_itself() {
    assert_eq!(
        decided(&walk(scene()), "/main/a", CONTAINER),
        ["padding-left"]
    );
}

/// The filed defect. The block declares `--pad-b` and nothing else, so a pass that required a
/// block to name a longhand never withdrew it and this element got no answer at all.
#[test]
fn names_the_longhand_a_token_only_block_decided() {
    assert_eq!(
        decided(&walk(scene()), "/main/b", CONTAINER),
        ["padding-left"]
    );
}

/// The same answer where *no* conditional block anywhere in the document spells the property.
/// The scene above keeps a longhand arm beside the token ones, so a pass that pooled every
/// block's declared names into one watch set would still watch `padding-left` and look
/// correct. Withdrawing that arm leaves the decided property nameable in no block at all.
#[test]
fn names_a_longhand_no_conditional_block_anywhere_spells() {
    let mut scene = scene();
    scene["sheets"][0][4]["rules"] = json!([
        { "selectorText": ".arm-b", "declarations": { "--pad-b": "62px" } },
        { "selectorText": ".frame", "declarations": { "--pad-d": "63px" } }
    ]);

    assert_eq!(
        decided(&walk(scene), "/main/b", CONTAINER),
        ["padding-left"]
    );
}

/// Custom properties inherit, so the reader need not be the element the token is declared on.
/// The block here sits on the container and the longhand it moves is a descendant's.
#[test]
fn names_a_descendant_longhand_an_inherited_token_decided() {
    assert_eq!(
        decided(&walk(scene()), "/main/d", CONTAINER),
        ["padding-left"]
    );
}

/// Both arms measured, for every carrier alike. Without the base the emitter can restore
/// nothing for the two token arms, because the authored text states only a reference.
#[test]
fn measures_both_arms_whichever_carrier_the_override_travelled_on() {
    let result = walk(scene());
    for (path, base) in [("/main/a", "4px"), ("/main/b", "5px"), ("/main/d", "6px")] {
        assert_eq!(
            record(&result, path)["condition_base"]["padding-left"],
            json!(base),
            "{path} lost the arm the unconditional cascade owes"
        );
    }
}

/// The anti-vacuity control for the token arms. A block whose token nothing reads decides
/// nothing, so an answer that credited every element under a live condition would fail here.
#[test]
fn credits_nothing_to_an_element_whose_bake_no_condition_moves() {
    let mut scene = scene();
    scene["elements"][3]["classes"] = json!(["unread"]);
    scene["sheets"][0][3]["selectorText"] = json!(".unread");
    scene["sheets"][0][3]["declarations"] = json!({ "padding-left": "63px" });

    assert!(chains(&walk(scene), "/main/d").is_empty());
}

/// The container itself bakes the second longhand of that block, and that one *is* spelled
/// there. Both must be answered by the same pass, or a repair aimed at tokens has replaced
/// one half of the answer with the other.
#[test]
fn answers_a_named_longhand_and_a_token_from_the_same_block() {
    let mut scene = scene();
    scene["elements"][0]["baked"] = json!({ "letter-spacing": "3px" });

    assert_eq!(
        decided(&walk(scene), "/main", CONTAINER),
        ["letter-spacing"]
    );
}
