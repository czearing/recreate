//! The single owner of the rules an element's generated boxes contribute.
//!
//! `::before` and `::after` are two slots of one kind. Every question the generator asks about
//! them — which slots an element uses, and what declarations each one emits — had been answered
//! separately at each site that asks, and the copies agreed on the same wrong answer: both wrote
//! the value from [`Pseudo::content`] and then the whole captured style map, which already
//! carries `content`, so every generated rule declared it twice.
//!
//! Naming the slots here once also gives the identity and the emitter one list to read, which
//! is what stops a third slot being added to one and forgotten in the other.

use crate::model::{Node, Pseudo, Styles};
use std::collections::BTreeMap;

/// The slots an element can decorate, in the order a rule set declares them, paired with what
/// this element put in each. The suffix is the selector's, so it is also what distinguishes the
/// two rules from one another.
pub fn slots(node: &Node) -> [(&'static str, Option<&Pseudo>); 2] {
    [
        ("::before", node.before.as_ref()),
        ("::after", node.after.as_ref()),
    ]
}

/// The declarations one generated box contributes.
///
/// `content` is taken from the field rather than from the style map, because it is the one
/// declaration that decides whether the box exists at all and a captured map need not carry it.
/// It is then dropped from the rest, so the rule declares it exactly once.
pub fn declarations(pseudo: &Pseudo, assets: &BTreeMap<String, String>) -> String {
    let rest: Styles = pseudo
        .style
        .iter()
        .filter(|(property, _)| property.as_str() != "content")
        .map(|(property, value)| (property.clone(), value.clone()))
        .collect();
    format!(
        "content:{};{}",
        pseudo.content,
        super::responsive::output_declarations(&rest, assets)
    )
}

/// Appends the rule for each slot this element actually uses, under `class`.
///
/// An element that declared one generated box receives one rule. Emitting both slots for every
/// class would fabricate a decoration on elements that never had one.
pub fn append(node: &Node, class: &str, assets: &BTreeMap<String, String>, css: &mut String) {
    for (suffix, pseudo) in slots(node) {
        if let Some(pseudo) = pseudo {
            css.push_str(&format!(
                ".{class}{suffix}{{{}}}\n",
                declarations(pseudo, assets)
            ));
        }
    }
}
