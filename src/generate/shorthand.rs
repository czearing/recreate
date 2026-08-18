//! Which longhand an authored declaration sets, when the author spelled it as a shorthand.
//!
//! A capture enumerates longhands, so `background` names no key in any sampled style while
//! `background-color` names one. Every stage that compares authored text against a sample has
//! to ask what the author's name stands for, and asking with the name alone silently misses
//! every shorthand a page is written with.

/// What an authored declaration `name: value` says about the longhand `property`.
pub(super) enum Claim<'a> {
    /// It does not set this longhand.
    Elsewhere,
    /// It sets it, to this value.
    Value(&'a str),
    /// It sets it, to a share of a value only the family's own grammar divides.
    Opaque,
}

/// A shorthand distributing one component gives that component to each longhand it sets, so
/// the component is what the longhand computed to and comparing it against a sample is exact.
/// Several components are divided by a per-family grammar this reads nothing of — two lengths
/// on a box are one per axis, two words on `font` are a size and a family — so the share is
/// named rather than guessed, and a caller that cannot name it keeps the answer it had.
pub(super) fn claim<'a>(name: &str, value: &'a str, property: &str) -> Claim<'a> {
    if name == property {
        return Claim::Value(value);
    }
    if !expands_to(name, property) {
        return Claim::Elsewhere;
    }
    match crate::model::value_components(value).as_slice() {
        [only] => Claim::Value(only),
        _ => Claim::Opaque,
    }
}

/// Every property of `style` that the declaration `name: value` set to the value measured
/// there — the properties for which this declaration is the proof that the condition
/// guarding it was in force.
///
/// The sample is the key space, so a name the author shortened is asked of the longhands it
/// stands for rather than looked up as itself and missed.
pub(super) fn measured(style: &crate::model::Styles, name: &str, value: &str) -> Vec<String> {
    style
        .iter()
        .filter(|(property, sample)| match claim(name, value, property) {
            Claim::Value(value) => *sample == value,
            Claim::Elsewhere | Claim::Opaque => false,
        })
        .map(|(property, _)| property.clone())
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
