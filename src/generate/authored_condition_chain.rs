//! One walk over an authored grouping at-rule, shared by everything that reads one.
//!
//! A grouping at-rule may hold another. A capture serialises a sheet carried by
//! `<link media="all">` as `@media all{...}`, so a page's real breakpoints arrive one group
//! inside another; a reader that stops at the outer group sees a selector list spelled
//! `@media (min-width: 544px)`, matches it against no element, and reports the page as having
//! no conditional rules at all — which on a real site is the whole population.
//!
//! Both readers need the same answer about that chain: the re-emitter must put every layer
//! back, and the withdrawal must know whether any layer can be false. Answering it once here is
//! what keeps a declaration the base rule gives up from being one the output states nowhere.

use super::authored_conditions::{document_answered, falsifiable};

/// The chain of document-answered conditions a style rule sits inside, innermost first.
///
/// Held on the stack rather than in a collection: the walk runs once per rule per node, and a
/// page carrying five figures of both pays for every allocation here that many times.
pub(super) struct Conditions<'a, 'p> {
    prelude: &'a str,
    outer: Option<&'p Conditions<'a, 'p>>,
}

impl Conditions<'_, '_> {
    /// The chain spelled as the text that opens it, outermost first — `@media a{@media b`.
    /// Kept as one string so a rule can be grouped by the conditions it sits under before it is
    /// spelled out, which is what lets rules sharing both be merged onto one selector list.
    pub(super) fn opening(&self) -> String {
        match self.outer {
            Some(outer) => format!("{}{{{}", outer.opening(), self.prelude),
            None => self.prelude.to_string(),
        }
    }

    /// Whether any layer of the chain has a false branch at all.
    pub(super) fn falsifiable(&self) -> bool {
        falsifiable(self.prelude) || self.outer.is_some_and(Conditions::falsifiable)
    }
}

/// Every style rule reached through nothing but document-answered conditions, with the chain it
/// sits inside.
pub(super) fn for_each_rule<'a>(
    rule: &'a str,
    visit: &mut dyn FnMut(&Conditions<'a, '_>, &'a str, &'a str),
) {
    // `@layer` is a carrier, so a condition rule authored inside one arrives still wrapped in
    // it. The layer is the rule's cascade position and is settled elsewhere; what this stage is
    // asking is whether a document-answered condition is present, so it reads through the
    // wrapper exactly as `css::global_rule` does.
    let (_, rule) = super::css_layers::peel(rule);
    let Some((prefix, body, _)) = super::css_scan::block(rule) else {
        return;
    };
    // The prelude travels verbatim rather than being taken apart and rebuilt, so a container
    // query's name and a `style()` query survive without this stage knowing the grammar of
    // either.
    let prelude = prefix.trim();
    if !document_answered(prelude) {
        return;
    }
    descend(
        &Conditions {
            prelude,
            outer: None,
        },
        body,
        visit,
    );
}

/// A block inside a grouping at-rule is either another at-rule or a style rule, and the two are
/// told apart by the one character CSS reserves for the first. An at-rule this stage does not
/// re-emit is not descended into, so nothing downstream can withdraw a branch behind a condition
/// the output never states.
fn descend<'a>(
    conditions: &Conditions<'a, '_>,
    mut body: &'a str,
    visit: &mut dyn FnMut(&Conditions<'a, '_>, &'a str, &'a str),
) {
    while let Some((head, inner, rest)) = super::css_scan::block(body) {
        body = rest;
        let head = head.trim();
        if head.starts_with('@') {
            if document_answered(head) {
                descend(
                    &Conditions {
                        prelude: head,
                        outer: Some(conditions),
                    },
                    inner,
                    visit,
                );
            }
            continue;
        }
        visit(conditions, head, inner);
    }
}
