//! Drops declarations that say nothing, without dropping the ones that say
//! "go back to the default".
//!
//! An undeclared property does not vanish — it takes a fallback value — so a
//! declaration is redundant exactly when it already equals that fallback. Which
//! value that is depends on whether the property inherits: a non-inherited one
//! falls back to its initial value, an inherited one to the parent's computed
//! value. Testing every property against its initial value therefore deletes
//! precisely the declarations that override a non-initial ancestor, because an
//! override back to the default is indistinguishable, by value alone, from never
//! having been declared. `visibility: visible` inside a `visibility: hidden`
//! ancestor is the case that costs a visible subtree rather than a shade.
//!
//! No CSSOM API reports whether a property inherits, so that fact has to be
//! written down. It is written down here, on the same row as the initial value,
//! so the two cannot drift apart and adding a property states both at once.

use crate::model::Styles;

/// What an element computes for a property it never declared.
#[derive(Clone, Copy)]
enum Fallback {
    /// Not inherited: an undeclared property starts over at its initial value.
    Initial,
    /// Inherited: an undeclared property takes whatever the parent computed.
    Parent,
}

use Fallback::{Initial, Parent};

/// Every property worth pruning, the value it falls back to when undeclared, and
/// where that fallback comes from.
const DEFAULTS: &[(&str, &str, Fallback)] = &[
    ("align-content", "normal", Initial),
    ("align-items", "normal", Initial),
    ("align-self", "auto", Initial),
    ("backdrop-filter", "none", Initial),
    ("background-blend-mode", "normal", Initial),
    ("background-clip", "border-box", Initial),
    ("background-image", "none", Initial),
    ("background-origin", "padding-box", Initial),
    ("background-position", "0% 0%", Initial),
    ("background-repeat", "repeat", Initial),
    ("background-size", "auto", Initial),
    ("border-collapse", "separate", Parent),
    ("border-spacing", "0px", Parent),
    ("box-shadow", "none", Initial),
    ("box-sizing", "content-box", Initial),
    ("clip-path", "none", Initial),
    ("column-gap", "normal", Initial),
    ("cursor", "auto", Parent),
    ("filter", "none", Initial),
    ("flex-basis", "auto", Initial),
    ("flex-direction", "row", Initial),
    ("flex-grow", "0", Initial),
    ("flex-shrink", "1", Initial),
    ("flex-wrap", "nowrap", Initial),
    ("float", "none", Initial),
    ("font-feature-settings", "normal", Parent),
    ("font-kerning", "auto", Parent),
    ("font-stretch", "100%", Parent),
    ("font-style", "normal", Parent),
    ("font-variation-settings", "normal", Parent),
    ("grid-auto-columns", "auto", Initial),
    ("grid-auto-flow", "row", Initial),
    ("grid-auto-rows", "auto", Initial),
    ("grid-column-end", "auto", Initial),
    ("grid-column-start", "auto", Initial),
    ("grid-row-end", "auto", Initial),
    ("grid-row-start", "auto", Initial),
    ("grid-template-areas", "none", Initial),
    ("grid-template-columns", "none", Initial),
    ("grid-template-rows", "none", Initial),
    ("justify-content", "normal", Initial),
    ("justify-items", "normal", Initial),
    ("justify-self", "auto", Initial),
    ("mask-image", "none", Initial),
    ("max-height", "none", Initial),
    ("max-width", "none", Initial),
    ("min-height", "auto", Initial),
    ("min-width", "auto", Initial),
    ("mix-blend-mode", "normal", Initial),
    ("object-fit", "fill", Initial),
    ("object-position", "50% 50%", Initial),
    ("opacity", "1", Initial),
    ("order", "0", Initial),
    ("overflow-x", "visible", Initial),
    ("overflow-y", "visible", Initial),
    ("pointer-events", "auto", Parent),
    ("position", "static", Initial),
    ("row-gap", "normal", Initial),
    ("scrollbar-color", "auto", Parent),
    ("scrollbar-gutter", "auto", Initial),
    ("scrollbar-width", "auto", Initial),
    ("table-layout", "auto", Initial),
    ("text-rendering", "auto", Parent),
    ("text-transform", "none", Parent),
    ("transform", "none", Initial),
    ("vertical-align", "baseline", Initial),
    ("visibility", "visible", Parent),
    ("white-space", "normal", Parent),
    ("word-break", "normal", Parent),
    ("z-index", "auto", Initial),
];

/// Removes each declaration that already equals what the element would compute
/// without it. `parent` is the element's inheritance parent — for a pseudo-element
/// that is its originating element, and for the document root it is `None`, where
/// the inheritance chain terminates in the initial value.
pub(super) fn remove_defaults(styles: &mut Styles, parent: Option<&Styles>) {
    for (name, initial, fallback) in DEFAULTS {
        let baseline = match fallback {
            Initial => initial,
            Parent => parent
                .and_then(|parent| parent.get(*name))
                .map_or(*initial, String::as_str),
        };
        if styles.get(*name).is_some_and(|current| current == baseline) {
            styles.remove(*name);
        }
    }
}

/// The parent values `remove_defaults` consults, as a signature fragment.
///
/// Two elements with byte-identical computed styles no longer emit identical
/// declarations: whether an inherited declaration survives depends on the parent.
/// Anything that dedupes elements by their style must fold this in, or it merges two
/// elements that need different rules and one of them silently takes the other's.
pub(super) fn inherited_context(parent: Option<&Styles>) -> String {
    let Some(parent) = parent else {
        return String::new();
    };
    DEFAULTS
        .iter()
        .filter(|(_, _, fallback)| matches!(fallback, Parent))
        .filter_map(|(name, _, _)| parent.get(*name).map(|value| format!("{name}:{value};")))
        .collect()
}

#[cfg(test)]
#[path = "declaration_defaults_tests.rs"]
mod tests;
