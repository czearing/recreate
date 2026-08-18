//! The relation between the name an author wrote and the longhands a capture records.

use super::{Claim, Shorthands, expands_to, renamed_parts, sets};
use crate::model::Styles;

/// No division recorded for any block — the fallback every hand-written fixture reaches.
fn undivided() -> Shorthands {
    Shorthands::new()
}

/// A block as the capture records it: the text, and the shares the engine stored under it.
fn divided(block: &str, shares: &[(&str, &str)]) -> Shorthands {
    Shorthands::from([(
        block.to_string(),
        shares
            .iter()
            .map(|(name, share)| ((*name).to_string(), (*share).to_string()))
            .collect(),
    )])
}

fn share<'a>(
    shorthands: &'a Shorthands,
    block: &str,
    name: &str,
    value: &'a str,
    property: &str,
) -> Option<&'a str> {
    match super::claim(shorthands, block, name, value, property) {
        Claim::Value(share) => Some(share),
        Claim::Elsewhere | Claim::Unsettled => None,
    }
}

fn value<'a>(name: &str, value: &'a str, property: &str) -> Option<&'a str> {
    match super::claim(no_division(), "", name, value, property) {
        Claim::Value(share) => Some(share),
        Claim::Elsewhere | Claim::Unsettled => None,
    }
}

fn no_division() -> &'static Shorthands {
    static EMPTY: std::sync::OnceLock<Shorthands> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Shorthands::new)
}

fn unsettled(name: &str, value: &str, property: &str) -> bool {
    matches!(
        super::claim(no_division(), "", name, value, property),
        Claim::Unsettled
    )
}

/// The name a capture records is the longhand, so the shorthand has to reach it by the
/// suffix rather than by a lookup that will not have the next family in it.
#[test]
fn reads_a_longhand_off_the_shorthand_that_prefixes_it() {
    assert!(expands_to("background", "background-color"));
    assert!(expands_to("transition", "transition-duration"));
    assert!(expands_to("padding", "padding-top"));
    assert!(!expands_to("background", "color"));
    assert!(!expands_to("padding", "margin-top"));
}

/// A name is not its own longhand by prefix, and must still answer for itself.
#[test]
fn answers_for_the_name_the_author_wrote() {
    assert!(!expands_to("color", "color"));
    assert_eq!(
        value("color", "rgb(1, 2, 3)", "color"),
        Some("rgb(1, 2, 3)")
    );
}

/// The families CSS renamed rather than prefixed are unreachable by suffix, so they are the
/// only thing written down.
#[test]
fn reads_the_families_whose_longhands_were_renamed() {
    assert!(expands_to("gap", "row-gap"));
    assert!(expands_to("inset", "left"));
    assert!(expands_to("place-items", "align-items"));
    assert!(expands_to("flex-flow", "flex-wrap"));
    assert_eq!(
        renamed_parts("gap"),
        Some(["row-gap", "column-gap"].as_ref())
    );
    assert_eq!(renamed_parts("padding"), None);
}

/// The suffix rule over-answers on purpose. `border` does not set `border-radius`, and the
/// engine's own division of the block is what refuses the pair — a block it divided is a
/// complete statement of the longhands that block sets, so a longhand missing from it is one
/// the declaration did not set.
#[test]
fn refuses_a_prefixed_property_the_engine_divided_the_block_without() {
    assert_eq!(value("border", "8px", "border-radius"), Some("8px"));

    let style = Styles::from([
        ("border-radius".to_string(), "4px".to_string()),
        ("border-top-width".to_string(), "8px".to_string()),
    ]);
    let block = "border: 8px solid red";
    let shorthands = divided(block, &[("border-top-width", "8px")]);

    assert_eq!(
        sets(&shorthands, block, &style, "border", "8px solid red"),
        ["border-top-width"]
    );
}

/// One component is what every longhand the shorthand sets computed to, so it transfers even
/// where nothing recorded the division. Anything longer is answered unsettled rather than
/// guessed: passing the whole text through publishes nonsense, and dividing it by position
/// publishes a *wrong* share, which is worse than none.
#[test]
fn transfers_one_component_and_refuses_to_divide_several_unaided() {
    assert_eq!(value("padding", "24px", "padding-top"), Some("24px"));
    assert!(unsettled("padding", "24px 8px", "padding-top"));
    assert!(unsettled(
        "background",
        "url(a.png) rgb(255, 0, 0)",
        "background-color"
    ));
}

/// A function call is one component however much whitespace it holds, or a colour would be
/// mistaken for a divided value and never transfer at all.
#[test]
fn counts_a_function_call_as_one_component() {
    assert_eq!(
        value("background", "rgb(255, 0, 0)", "background-color"),
        Some("rgb(255, 0, 0)")
    );
}

/// Which longhands a declaration names is a question about names and the engine's division of
/// the block, never about whether the sample happens to hold the authored text: an override
/// spelled `0.5em` sets `padding-left` exactly as one spelled `8px` does. A property under an
/// unrelated name is still not reported, because no spelling of `background` reaches `color`.
#[test]
fn reports_the_longhands_the_declaration_names_however_its_value_is_spelled() {
    let style = Styles::from([
        ("padding-left".to_string(), "8px".to_string()),
        ("color".to_string(), "rgb(0, 255, 0)".to_string()),
    ]);

    assert_eq!(
        sets(&undivided(), "", &style, "padding-left", "0.5em"),
        ["padding-left"]
    );
    assert_eq!(
        sets(&undivided(), "", &style, "background", "rgb(0, 255, 0)"),
        [] as [String; 0]
    );
}

#[path = "shorthand_division_tests.rs"]
mod division;
