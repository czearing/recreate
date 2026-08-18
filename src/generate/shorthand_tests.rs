//! The relation between the name an author wrote and the longhands a capture records.

use super::{Claim, expands_to, measured, renamed_parts};
use crate::model::Styles;

fn value<'a>(name: &str, value: &'a str, property: &str) -> Option<&'a str> {
    match super::claim(name, value, property) {
        Claim::Value(share) => Some(share),
        Claim::Elsewhere | Claim::Opaque => None,
    }
}

fn opaque(name: &str, value: &str, property: &str) -> bool {
    matches!(super::claim(name, value, property), Claim::Opaque)
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

/// The suffix rule over-answers on purpose. `border` does not set `border-radius`, and what
/// refuses the pair is the value the caller compares, so this reports the claim and lets the
/// comparison decide.
#[test]
fn claims_a_prefixed_property_the_shorthand_does_not_set() {
    assert_eq!(value("border", "8px", "border-radius"), Some("8px"));

    let style = Styles::from([
        ("border-radius".to_string(), "4px".to_string()),
        ("border-top-width".to_string(), "8px".to_string()),
    ]);

    assert_eq!(measured(&style, "border", "8px"), ["border-top-width"]);
}

/// One component is what every longhand the shorthand sets computed to, so it transfers.
/// Several are divided by a grammar this reads nothing of, so the share is named undecodable
/// rather than guessed — passing the whole text through publishes nonsense.
#[test]
fn transfers_one_component_and_refuses_to_divide_several() {
    assert_eq!(value("padding", "24px", "padding-top"), Some("24px"));
    assert!(opaque("padding", "24px 8px", "padding-top"));
    assert!(opaque(
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

/// Only a property whose measured value the declaration accounts for is reported, so a
/// property that merely happens to hold the same value under an unrelated name is not.
#[test]
fn reports_only_the_properties_the_declaration_accounts_for() {
    let style = Styles::from([
        ("background-color".to_string(), "rgb(0, 255, 0)".to_string()),
        ("color".to_string(), "rgb(0, 255, 0)".to_string()),
        ("background-image".to_string(), "none".to_string()),
    ]);

    assert_eq!(
        measured(&style, "background", "rgb(0, 255, 0)"),
        ["background-color"]
    );
}
