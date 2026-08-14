//! The single owner of the rules an element's generated boxes contribute.
//!
//! `::before`, `::after` and `::backdrop` are slots of one kind. Every question the generator
//! asks about them — which slots an element uses, and what declarations each one emits — had
//! been answered separately at each site that asks, and the copies agreed on the same wrong
//! answer: both wrote the value from [`Pseudo::content`] and then the whole captured style
//! map, which already carries `content`, so every generated rule declared it twice.
//!
//! The slots now come from the record itself rather than from a list written here, so a slot
//! the capture learns to read needs no edit in this file and none at any reader. Naming them
//! in one place is still what stops a slot being added to the identity and forgotten in the
//! emitter.

use crate::model::{Node, Pseudo, Pseudos, Styles};
use std::collections::{BTreeMap, BTreeSet};

/// The slots this element used, in the order a rule set declares them, paired with what it
/// put in each. The key is the selector's suffix, so it is also what distinguishes the rules.
pub fn slots(node: &Node) -> impl Iterator<Item = (&str, &Pseudo)> {
    node.pseudos
        .iter()
        .map(|(suffix, pseudo)| (suffix.as_str(), pseudo))
}

/// Every slot either side used, paired with what each holds there. A slot one side dropped
/// appears with `None` on that side, which is the case a rule must still emit — a box that
/// stopped existing has to be turned off rather than left standing.
pub fn paired<'a>(
    base: &'a Pseudos,
    current: &'a Pseudos,
) -> Vec<(&'a str, Option<&'a Pseudo>, Option<&'a Pseudo>)> {
    base.keys()
        .chain(current.keys())
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|suffix| (suffix, base.get(suffix), current.get(suffix)))
        .collect()
}

/// The declarations one generated box contributes.
///
/// `content` is taken from the field rather than from the style map, because for a box that
/// exists only because `content` produced it, that value decides whether the box exists at
/// all and a captured map need not carry it. It is then dropped from the rest, so the rule
/// declares it exactly once. A box the user agent generates has no `content` of its own, and
/// declaring one would assert something the page never said.
pub fn declarations(pseudo: &Pseudo, assets: &BTreeMap<String, String>) -> String {
    let rest: Styles = pseudo
        .style
        .iter()
        .filter(|(property, _)| property.as_str() != "content")
        .map(|(property, value)| (property.clone(), value.clone()))
        .collect();
    format!(
        "{}{}",
        content_declaration(&pseudo.content, assets),
        super::responsive::output_declarations(&rest, assets)
    )
}

/// `content` spelled as a declaration, or nothing when the box carries none.
///
/// This is the one place that answers what text a box's `content` contributes, so it is also
/// the one place that localises it. The value arrives resolved against the capture origin —
/// a `<url>` in a computed value is an absolute URL — and every other declaration reaches the
/// recreation through the same rewrite. A caller that needs the value for anything the
/// recreation will read must come through here rather than spell the rule again.
pub fn content_declaration(content: &str, assets: &BTreeMap<String, String>) -> String {
    if content.is_empty() {
        return String::new();
    }
    format!("content:{};", super::asset_urls::rewrite(content, assets))
}

/// Appends the rule for each slot this element actually used, under `class`.
///
/// An element that declared one generated box receives one rule. Emitting every slot for
/// every class would fabricate a decoration on elements that never had one.
pub fn append(node: &Node, class: &str, assets: &BTreeMap<String, String>, css: &mut String) {
    for (suffix, pseudo) in slots(node) {
        let declarations = declarations(pseudo, assets);
        if !declarations.is_empty() {
            css.push_str(&format!(".{class}{suffix}{{{declarations}}}\n"));
        }
    }
}
