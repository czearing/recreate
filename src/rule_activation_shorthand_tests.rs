//! What the capture records about how the engine divided each authored declaration block.
//!
//! The division cannot be recovered later. Generation runs from the artifact alone — a
//! separate entry point with no page, no server and no engine — so a share the capture did not
//! write down is a share that is gone. These run the real walk over a scripted CSSOM, so what
//! they assert is what a browser's own rule objects would put in the artifact.

use super::{style, walk};
use serde_json::{Value, json};

/// A block spelled as a shorthand, alongside the longhand and one-component spellings of the
/// same declaration, so what is recorded and what is not are both observable from one run.
fn scene() -> Value {
    json!({
        "elements": [{ "path": "/main/div", "classes": ["multi", "single", "longhand"] }],
        "matching": {},
        "sheets": [[
            {
                "selectorText": ".multi",
                "declarations": { "background": "padding-box padding-box rgb(255, 0, 0)" },
                "expanded": {
                    "background-clip": "padding-box",
                    "background-color": "rgb(255, 0, 0)",
                    "background-image": "initial",
                    "background-origin": "padding-box"
                }
            },
            style(".single", "background-color", "rgb(255, 0, 0)"),
            {
                "selectorText": ".fluid",
                "declarations": { "padding": "0px var(--gutter)" },
                "expanded": {
                    "padding-bottom": "",
                    "padding-left": "",
                    "padding-right": "",
                    "padding-top": ""
                }
            }
        ]]
    })
}

fn recorded(result: &Value) -> std::collections::BTreeMap<String, Value> {
    result["shorthands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|block| {
            (
                block["text"].as_str().unwrap().to_string(),
                block["shares"].clone(),
            )
        })
        .collect()
}

/// The filed defect's acquisition half. Without this the generator is asked to divide a value
/// whose grammar only the engine reads, and every answer it can give is a guess.
#[test]
fn records_the_longhands_a_shorthand_block_actually_set() {
    let blocks = recorded(&walk(scene()));
    let shares = blocks
        .get("background: padding-box padding-box rgb(255, 0, 0);")
        .unwrap_or_else(|| panic!("no division was recorded for the shorthand block: {blocks:?}"));

    assert_eq!(shares["background-color"], "rgb(255, 0, 0)");
    assert_eq!(shares["background-origin"], "padding-box");
}

/// The omitted components of a shorthand are reported as `initial`, which is an instruction to
/// the cascade rather than a value. Recording them would be most of the artifact's new weight
/// and none of its new information, and would publish `initial` as though the author wrote it.
#[test]
fn refuses_the_css_wide_keywords_an_omitted_component_reports() {
    let blocks = recorded(&walk(scene()));
    let shares = &blocks["background: padding-box padding-box rgb(255, 0, 0);"];

    assert!(
        shares.get("background-image").is_none(),
        "an omitted component was recorded as a declared value: {shares:?}"
    );
}

/// A share the engine could not settle is present and blank. That is the difference between
/// "the author wrote no arm" and "the author wrote one nothing can read yet", and a later
/// stage deletes a declaration on the first answer and not the second.
#[test]
fn records_an_unsettled_share_rather_than_omitting_it() {
    let blocks = recorded(&walk(scene()));
    let shares = &blocks["padding: 0px var(--gutter);"];

    assert_eq!(shares["padding-top"], "");
    assert_eq!(shares["padding-left"], "");
}

/// A block that sets only what it names tells a later stage nothing its own text does not.
/// Recording it would be size without information, and the test that decides is the engine's
/// — a longhand stored under a name the block never mentions — rather than a list of families.
#[test]
fn records_nothing_for_a_block_that_declares_no_shorthand() {
    let blocks = recorded(&walk(scene()));

    assert!(
        !blocks.contains_key("background-color: rgb(255, 0, 0);"),
        "a block with no shorthand was recorded: {blocks:?}"
    );
}
