//! Which longhand an authored declaration sets, when the author spelled it as a shorthand.
//!
//! A capture enumerates longhands, so `background` names no key in any sampled style while
//! `background-color` names one. Every stage that compares authored text against a sample has
//! to ask what the author's name stands for, and asking with the name alone silently misses
//! every shorthand a page is written with.
//!
//! The question has two halves and only one of them can be read off the text. Which longhands
//! a name *may* set follows from how CSS spells them, so `expands_to` answers it for families
//! nobody listed. What share each one gets is a per-family grammar, and the only reader of
//! that grammar is the engine that parsed the sheet — so the capture records the engine's own
//! division and this stage looks it up, rather than transcribing a table of families that
//! answers for the ones somebody thought of and withholds the rest.

use std::collections::BTreeMap;

/// How the engine divided each authored declaration block, keyed by the block text.
pub type Shorthands = BTreeMap<String, BTreeMap<String, String>>;

/// What an authored declaration `name: value` says about the longhand `property`.
pub(super) enum Claim<'a> {
    /// It does not set this longhand.
    Elsewhere,
    /// It sets it, to this value.
    Value(&'a str),
    /// It sets it, to a share nothing can state — because the engine itself reported the
    /// division as unsettled, or because no division was recorded for the block at all.
    Unsettled,
}

/// The block text a recorded division is keyed by, from a declarations run as either the rule
/// table or the condition walk hands it over. One spelling, so a lookup cannot miss by a brace.
pub(super) fn block_key(declarations: &str) -> &str {
    declarations.trim().trim_end_matches('}').trim()
}

/// The engine's share of `block` for `property`, where the capture recorded one.
fn divided<'a>(shorthands: &'a Shorthands, block: &str, property: &str) -> Option<&'a str> {
    shorthands
        .get(block_key(block))?
        .get(property)
        .map(String::as_str)
}

/// What the declaration `name: value`, written in `block`, says about `property`.
///
/// The engine's division wins wherever the capture recorded one: it is that block's own
/// parse, already serialised in the vocabulary a sample uses, and it has resolved the block's
/// internal cascade, so a longhand restated after its shorthand reads back as the restatement.
///
/// Where no division was recorded — an artifact older than the recording, or a fixture written
/// by hand — a single-component value is still exact, because a shorthand with one component
/// has only that component to give. Anything longer is answered `Unsettled`, which withdraws
/// nothing and deletes nothing.
pub(super) fn claim<'a>(
    shorthands: &'a Shorthands,
    block: &str,
    name: &str,
    value: &'a str,
    property: &str,
) -> Claim<'a> {
    if name == property {
        return Claim::Value(value);
    }
    if !expands_to(name, property) {
        return Claim::Elsewhere;
    }
    match divided(shorthands, block, property) {
        Some("") => Claim::Unsettled,
        Some(share) => Claim::Value(share),
        // The engine divided this block and did not store this longhand, so the block does
        // not set it. That refusal is what bounds `expands_to`'s deliberate over-answer —
        // `border` prefixes `border-radius` and sets none of it — without a second list.
        None if shorthands.contains_key(block_key(block)) => Claim::Elsewhere,
        None => match crate::model::value_components(value).as_slice() {
            [only] => Claim::Value(only),
            _ => Claim::Unsettled,
        },
    }
}

/// Every property of `style` that the declaration `name: value` of `block` sets.
///
/// The sample is the key space, so a name the author shortened is asked of the longhands it
/// stands for rather than looked up as itself and missed.
///
/// This answers which longhands a declaration *names*, and deliberately not whether it is
/// the one that produced their samples. The two questions used to be one, answered by
/// comparing the authored text against the sample — which holds only where the author spelled
/// a value the way the engine serialises it, and silently fails for every relative length,
/// percentage, math function, container unit and colour keyword. Whether a declaration
/// produced a sample is the engine's answer, carried on [`crate::model::Node::condition_decided`].
pub(super) fn sets(
    shorthands: &Shorthands,
    block: &str,
    style: &crate::model::Styles,
    name: &str,
    value: &str,
) -> Vec<String> {
    style
        .keys()
        .filter(|property| {
            !matches!(
                claim(shorthands, block, name, value, property),
                Claim::Elsewhere
            )
        })
        .cloned()
        .collect()
}

/// Whether `property` is one of the longhands `name` is the shorthand for.
///
/// CSS spells a longhand as its shorthand's own name and a suffix in every family but the few
/// whose parts were named before the shorthand that gathers them, so the rule is read off the
/// names and only the renames are written down. Reading it this way is what admits a family
/// nobody listed. It over-answers — `border` prefixes `border-radius`, which it does not set —
/// and the value the caller compares is what refuses such a pair, never a longer list.
pub(super) fn expands_to(name: &str, property: &str) -> bool {
    property
        .strip_prefix(name)
        .is_some_and(|rest| rest.starts_with('-'))
        || renamed_parts(name).is_some_and(|parts| parts.contains(&property))
}

/// The longhands of the shorthands CSS did not spell as their own prefix.
pub(super) fn renamed_parts(name: &str) -> Option<&'static [&'static str]> {
    RENAMED
        .iter()
        .find(|(shorthand, _)| *shorthand == name)
        .map(|(_, parts)| *parts)
}

const RENAMED: &[(&str, &[&str])] = &[
    ("flex-flow", &["flex-direction", "flex-wrap"]),
    ("gap", &["row-gap", "column-gap"]),
    ("inset", &["top", "right", "bottom", "left"]),
    ("place-content", &["align-content", "justify-content"]),
    ("place-items", &["align-items", "justify-items"]),
    ("place-self", &["align-self", "justify-self"]),
];

#[cfg(test)]
#[path = "shorthand_tests.rs"]
mod tests;
