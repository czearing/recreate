//! What the unconditional rule owes a property a re-published condition decided.
//!
//! The withdrawal and the band are two halves of one answer, so both read this module: the
//! base rule states the arm below the condition and the band states the override, and a
//! property this cannot settle is left in neither by both.

use super::authored_conditions::withdrawable;
use super::authored_css_index::Index;
use crate::model::{Node, Styles};
use std::collections::{BTreeMap, BTreeSet};

/// The value the base rule owes when a document-answered condition supplied the one measured.
///
/// A capture reads each element once per sampled viewport, so every declaration it records is
/// the branch the conditions happened to be on. The prelude is re-emitted above, which puts
/// that branch back wherever the condition holds — but nothing had removed it from the base
/// rule, so the recreation stated the override twice and stated the arm below the breakpoint
/// nowhere, painting the override at every width.
///
/// *Which* properties a condition decided is the engine's answer, taken while the page was
/// open by withdrawing the blocks of the rules re-emitted above and reading what moved; it is
/// on the node as [`Node::condition_decided`], credited to the chain that moved it. Nothing
/// here re-derives that set from the authored text — a conditional block declaring only a
/// custom property decides longhands it never names, so a candidate list read off the text
/// answers for the ones somebody spelled and withholds every one carried by a token.
///
/// What replaces the withdrawn value is the unconditional cascade's own last word, or nothing
/// where the author wrote none — below the breakpoint the element takes its inherited or
/// initial value, which the recreation re-produces by saying nothing. Where the author did
/// write an arm the text cannot state — a reference this stage resolves elsewhere, or a share
/// of a shorthand it cannot divide — the engine's measurement of that same arm stands in, so
/// a value is dropped only when there was none. No width is consulted, so this is equally the
/// answer for a container query, whose condition no viewport can settle at all.
pub fn restore_unconditional(styles: &mut Styles, node: &Node, index: &Index<'_>) {
    let names = withdrawable(node)
        .flat_map(|(_, properties)| properties)
        .cloned()
        .collect();
    for (name, arm) in arms(node, index, names) {
        match arm {
            Arm::Value(value) => {
                styles.insert(name, value);
            }
            Arm::Drop => {
                styles.remove(&name);
            }
            Arm::Keep => (),
        }
    }
}

/// What the base rule owes a withdrawn property.
pub(super) enum Arm {
    /// The arm below the condition, to be stated unconditionally while the band restates the
    /// override.
    Value(String),
    /// No arm: the author declared the property nowhere unconditional. Either the source
    /// takes its inherited or initial value below the condition, which the recreation
    /// re-produces by saying nothing, or the property is a used value the box settled — a
    /// width that moved because a padding did — which layout re-derives from what did change.
    /// Neither arm may state it, or the recreation would pin a measurement to a rule.
    Drop,
    /// An arm the author wrote and this stage cannot state, and that nothing measured. The
    /// baked override stays rather than be replaced by an initial value the source never
    /// takes; the band adds nothing, because the class already carries that same value.
    Keep,
}

/// What the unconditional rule owes each of `names`, which the caller has already narrowed to
/// properties this node bakes. Read once and shared by both directions, so the band restates
/// exactly the overrides the base rule gave up.
pub(super) fn arms(
    node: &Node,
    index: &Index<'_>,
    names: BTreeSet<String>,
) -> BTreeMap<String, Arm> {
    if names.is_empty() {
        return BTreeMap::new();
    }
    // Asked once for the whole set rather than once per property: the unconditional cascade
    // is resolved by walking this node's rules, and walking them per declaration is what a
    // page with five figures of both costs most.
    let unconditional = index.unconditional_values(node, &names);
    names
        .into_iter()
        .map(|name| {
            let arm = match unconditional.get(&name) {
                Some(Some(value)) if !super::authored_css_rules::deferred_binding(value) => {
                    Arm::Value(value.clone())
                }
                // Declared, in words this stage cannot state: a share of a shorthand it could
                // not divide, or a reference whose token another layer publishes with no arm
                // of its own. The engine measured that same arm while the page was open, so
                // it can stand in where it exists.
                Some(_) => match node.condition_base.get(&name) {
                    Some(measured) => Arm::Value(measured.clone()),
                    None => Arm::Keep,
                },
                None => Arm::Drop,
            };
            (name, arm)
        })
        .collect()
}
