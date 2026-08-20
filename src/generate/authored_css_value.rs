//! Reading the authored value of a single property, as opposed to the whole declaration set.
//!
//! Every reader here consults the unconditional rules alone. What the author wrote inside a
//! condition is a different question with a different owner, because the recreation re-emits
//! that condition rather than folding it into the base rule.

use super::authored_css_table::Table;
use super::shorthand::{Claim, Shorthands};
use crate::model::Node;

/// What the authored sheet says about one property on one element.
///
/// The three answers are distinct because deleting on the wrong one is silent. "The author
/// declared nothing" licenses removing a sampled measurement; "the author declared something
/// this stage could not read" does not, because removing it publishes the property's initial
/// value in place of one the source never took.
pub(super) enum Declared {
    /// No rule this node matches declares the property.
    Absent,
    /// A rule declares it, through a value this stage cannot divide into longhands.
    Unreadable,
    /// A rule declares it, and this is the value to emit.
    Value(String),
}

/// A value made only of absolute pixel lengths. It resolves to itself, so comparing it
/// against the captured computed value is exact — unlike `1fr`, `auto`, or a percentage,
/// which resolve against the layout and legitimately differ from the sample.
pub(super) fn absolute_length(value: &str) -> bool {
    !value.trim().is_empty()
        && value.split_whitespace().all(|token| {
            token == "0"
                || token
                    .strip_suffix("px")
                    .is_some_and(|number| number.parse::<f32>().is_ok())
        })
}

/// The last authored value for a property, with none of the filtering the whole-declaration
/// reader applies. A size written as `var(--card-width)` or `clamp(...)` is dropped there
/// because it cannot be compared against the sampled value, and treating it as absent
/// leaves the sample in its place — which pins the box to the captured viewport. The
/// authored text is what the source actually says, so it is what gets emitted.
///
/// Names are resolved to their physical equivalent first. A source that writes
/// `max-inline-size` is authoring `max-width`, and a literal name comparison reports
/// it as unauthored — which deletes the declaration instead of keeping it.
///
/// CSS-wide keywords are skipped. They declare no value, so emitting one only
/// clobbers a correct value the generator wrote in a lower-precedence rule.
///
/// Several rules may declare the same property, and this table models neither
/// `@layer` order nor specificity, so the textually last declaration is not
/// reliably the cascade winner. The captured computed value settles it: a
/// candidate that is a concrete literal disagreeing with the sample demonstrably
/// lost, and emitting it replaces correct geometry with a losing declaration.
/// Fluent gives a card `padding: var(--component-card-padding)` while the page
/// also authors `.card { padding: 0px }`; the card computes to 12px, so the
/// literal lost and only the custom-property reference may be emitted.
pub(super) fn authored(
    table: &Table<'_>,
    shorthands: &Shorthands,
    node: &Node,
    property: &str,
) -> Option<String> {
    match declared(table, shorthands, node, property) {
        Declared::Value(value) => Some(value),
        Declared::Absent | Declared::Unreadable => None,
    }
}

/// The same reading, with the answer the `Option` above cannot carry.
pub(super) fn declared(
    table: &Table<'_>,
    shorthands: &Shorthands,
    node: &Node,
    property: &str,
) -> Declared {
    let (candidates, unreadable) = candidates(table, shorthands, node, property);
    let Some(last) = candidates.last() else {
        return if unreadable {
            Declared::Unreadable
        } else {
            Declared::Absent
        };
    };
    let Some(sampled) = node.style.get(property) else {
        return Declared::Value(last.clone());
    };
    if let Some(agreeing) = candidates.iter().rev().find(|value| *value == sampled) {
        return Declared::Value(agreeing.clone());
    }
    if absolute_length(last) {
        return Declared::Value(sampled.clone());
    }
    Declared::Value(last.clone())
}

/// Every authored value for `property` on this node, in cascade order, and whether any rule
/// declared it through a value that produced none.
///
/// A shorthand is asked of the engine's own division of the block, recorded by the capture,
/// rather than of a table of families written here — the same source the whole-declaration
/// reader already trusts. Without it a sheet is read only for the names it happens to spell,
/// and CSSOM reserialises a complete set of longhands back into its shorthand, so the spelling
/// a page was authored in is not the spelling that arrives.
fn candidates(
    table: &Table<'_>,
    shorthands: &Shorthands,
    node: &Node,
    property: &str,
) -> (Vec<String>, bool) {
    let mut values = Vec::new();
    let mut unreadable = false;
    for block in table.blocks(node) {
        for (name, value) in super::css_declaration::parsed(block) {
            let value = value.trim().trim_end_matches('}').trim();
            if value.is_empty() || super::authored_css_rules::cascade_keyword(value) {
                continue;
            }
            if super::authored_css_rules::physical_property(node, name).answers(name, property) {
                values.push(value.to_string());
                continue;
            }
            match super::shorthand::claim(shorthands, block, name, value, property) {
                Claim::Value(share) => values.push(share.to_string()),
                Claim::Unsettled => unreadable = true,
                Claim::Elsewhere => (),
            }
        }
    }
    (values, unreadable)
}

/// The last authored value for `property` parsed as a positive integer, for the attributes
/// a source states in CSS rather than in markup.
pub(super) fn positive_integer(table: &Table<'_>, node: &Node, property: &str) -> Option<u32> {
    table
        .declarations_of(node)
        .filter(|(name, _)| *name == property)
        .map(|(_, value)| {
            value
                .trim_end_matches('}')
                .trim()
                .trim_end_matches("!important")
                .trim()
        })
        .next_back()?
        .parse()
        .ok()
        .filter(|value| *value > 0)
}

/// Restores the value a stylesheet actually authored for `property`, in place of the
/// computed value the browser baked. Selector shape decides which elements a rule reaches,
/// never whether a matched declaration is real, so this uses the same matcher as every
/// other direct lookup rather than a class-keyed one of its own.
///
/// A candidate whose binding is deferred disqualifies the answer instead of leaving the
/// ballot. Excluding it does not make the agreement test cautious, it makes it blind: the
/// test can only weigh what it is handed, so removing the dissenting candidate manufactures
/// the unanimity it looks for and certifies the declaration that candidate defeated.
///
/// Agreement is the whole test because this table sorts candidates by cascade layer and
/// models neither specificity nor importance, so where candidates disagree no position in
/// the list names the winner. Abstaining costs nothing: the engine already resolved the
/// cascade and the capture holds its answer, which the caller then leaves in place.
pub(super) fn inherited(table: &Table<'_>, node: &Node, property: &str) -> Option<String> {
    let values = table
        .declarations_of(node)
        .filter(|(name, _)| *name == property)
        .map(|(_, value)| value)
        .collect::<Vec<_>>();
    if values
        .iter()
        .any(|value| super::authored_css_rules::deferred_binding(value))
    {
        return None;
    }
    // Every candidate equals the first, so which end is read is immaterial by construction
    // rather than by convention.
    let first = values.first()?;
    values
        .iter()
        .all(|value| value == first)
        .then(|| (*first).to_string())
}

#[cfg(test)]
#[path = "inherited_vote_tests.rs"]
mod vote_tests;
