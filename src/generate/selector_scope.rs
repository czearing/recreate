//! Authored selectors rewritten onto generated classes.
//!
//! Every authored declaration reaches an element in the recreation through a generated
//! class, so an authored selector cannot be copied out verbatim — the author's own tokens
//! are not in the emitted markup. For a single compound the rewrite is trivial and the
//! generator has always done it. For a selector carrying a combinator it was not done at
//! all, and the rule was dropped, because one class on the subject cannot encode "has a
//! `.theme` ancestor": that requirement lives between two elements, not on one.
//!
//! The rewrite that keeps it expresses each compound as a marker class carried by exactly
//! the elements that compound matches, leaving the combinators alone, so `.theme .card`
//! becomes `.<theme> .<card>`. Compound count is unchanged, so specificity is unchanged, and
//! the relationship is still expressed as a relationship. This is the scoping transform CSS
//! Modules performs for the same reason, and it borrows that transform's guarantee rather
//! than only its shape: the marker is minted from the authored compound, so it is injective
//! over the distinctions the author drew. The generated paint class is not — it is an
//! equivalence class over computed style, shared by every element that paints alike — so a
//! selector built from paint classes would reach the look-alikes the author excluded. A
//! selector of one compound needs no relationship and keeps using the paint class, which is
//! why a page carrying no combinator selector gains no markers at all.
//!
//! Resolution follows the engine's own right-to-left order: the subject is tested first and
//! a node that fails it costs nothing more, so no ancestor is walked for a rule that was
//! never going to match. Where several ancestors satisfy a compound the nearest is taken —
//! any of them yields a selector that matches this node, and the nearest is the tightest.

use super::compound::{matches_node, split};
use super::selector_marker::name as marker;
use crate::model::Node;
use std::collections::{BTreeMap, HashMap};

/// A rewritten selector and the authored compounds whose markers it is built from.
pub(super) struct Scoped {
    pub(super) selector: String,
    pub(super) compounds: Vec<String>,
}

/// The captured tree and the class the generator assigned to each of its nodes.
pub(super) struct Scope<'a> {
    by_path: HashMap<&'a str, &'a Node>,
    classes: &'a BTreeMap<String, String>,
    order: HashMap<&'a str, usize>,
    prefix: &'a str,
}

impl<'a> Scope<'a> {
    pub(super) fn new(
        nodes: &'a [Node],
        classes: &'a BTreeMap<String, String>,
        prefix: &'a str,
    ) -> Self {
        Self {
            by_path: nodes
                .iter()
                .map(|node| (node.path.as_str(), node))
                .collect(),
            classes,
            order: nodes
                .iter()
                .enumerate()
                .map(|(index, node)| (node.path.as_str(), index))
                .collect(),
            prefix,
        }
    }

    pub(super) fn class(&self, node: &Node) -> Option<&str> {
        self.classes.get(&node.path).map(String::as_str)
    }

    /// The selector rewritten for this node, or `None` when it does not match it.
    pub(super) fn rewrite(&self, selector: &str, node: &'a Node) -> Option<Scoped> {
        let mut steps = split(selector).into_iter().rev();
        let (mut relation, subject) = steps.next()?;
        if !matches_node(subject, node) {
            return None;
        }
        let ancestors: Vec<_> = steps.collect();
        if ancestors.is_empty() {
            return Some(Scoped {
                selector: format!(".{}", self.class(node)?),
                compounds: Vec::new(),
            });
        }
        let mut emitted = format!(".{}", marker(self.prefix, subject));
        let mut compounds = vec![subject.to_string()];
        let mut current = node;
        for (combinator, compound) in ancestors {
            let relationship = relation?;
            current = self.relative(current, relationship, compound)?;
            emitted = format!(".{}{relationship}{emitted}", marker(self.prefix, compound));
            compounds.push(compound.to_string());
            relation = combinator;
        }
        Some(Scoped {
            selector: emitted,
            compounds,
        })
    }

    /// The nearest node standing in the named relationship to `node` that matches `compound`.
    fn relative(&self, node: &'a Node, combinator: char, compound: &str) -> Option<&'a Node> {
        match combinator {
            '>' => self
                .parent(node)
                .filter(|parent| matches_node(compound, parent)),
            ' ' => {
                let mut ancestor = self.parent(node);
                while let Some(current) = ancestor {
                    if matches_node(compound, current) {
                        return Some(current);
                    }
                    ancestor = self.parent(current);
                }
                None
            }
            '+' => self
                .preceding(node)
                .next()
                .filter(|sibling| matches_node(compound, sibling)),
            _ => self
                .preceding(node)
                .find(|sibling| matches_node(compound, sibling)),
        }
    }

    fn parent(&self, node: &Node) -> Option<&'a Node> {
        self.by_path.get(node.parent.as_deref()?).copied()
    }

    /// This node's element siblings, nearest first. Text nodes carry no class and cannot be
    /// named by a selector, so they are not siblings for this purpose.
    fn preceding(&self, node: &'a Node) -> impl Iterator<Item = &'a Node> {
        let position = self.order.get(node.path.as_str()).copied().unwrap_or(0);
        let mut siblings = self
            .by_path
            .values()
            .copied()
            .filter(|candidate| {
                candidate.parent == node.parent
                    && candidate.tag != "#text"
                    && self
                        .order
                        .get(candidate.path.as_str())
                        .copied()
                        .unwrap_or(0)
                        < position
            })
            .collect::<Vec<_>>();
        siblings.sort_by_key(|candidate| {
            std::cmp::Reverse(
                self.order
                    .get(candidate.path.as_str())
                    .copied()
                    .unwrap_or(0),
            )
        });
        siblings.into_iter()
    }
}
