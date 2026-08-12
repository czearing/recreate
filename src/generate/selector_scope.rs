//! Authored selectors rewritten onto generated classes.
//!
//! Every authored declaration reaches an element in the recreation through a generated
//! class, so an authored selector cannot be copied out verbatim — the author's own tokens
//! are not in the emitted markup. For a single compound the rewrite is trivial and the
//! generator has always done it. For a selector carrying a combinator it was not done at
//! all, and the rule was dropped, because one class on the subject cannot encode "has a
//! `.theme` ancestor": that requirement lives between two elements, not on one.
//!
//! The rewrite that keeps it maps each compound onto the generated class of the node that
//! compound matched and leaves the combinators alone, so `.theme .card` becomes
//! `.<theme> .<card>`. Compound count is unchanged, so specificity is unchanged, and the
//! relationship is still expressed as a relationship. This is the scoping transform CSS
//! Modules performs for the same reason.
//!
//! Resolution follows the engine's own right-to-left order: the subject is tested first and
//! a node that fails it costs nothing more, so no ancestor is walked for a rule that was
//! never going to match. Where several ancestors satisfy a compound the nearest is taken —
//! any of them yields a selector that matches this node, and the nearest is the tightest.

use super::compound::matches_node;
use crate::model::Node;
use std::collections::{BTreeMap, HashMap};

/// The captured tree and the class the generator assigned to each of its nodes.
pub(super) struct Scope<'a> {
    by_path: HashMap<&'a str, &'a Node>,
    classes: &'a BTreeMap<String, String>,
    order: HashMap<&'a str, usize>,
}

impl<'a> Scope<'a> {
    pub(super) fn new(nodes: &'a [Node], classes: &'a BTreeMap<String, String>) -> Self {
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
        }
    }

    pub(super) fn class(&self, node: &Node) -> Option<&str> {
        self.classes.get(&node.path).map(String::as_str)
    }

    /// The selector rewritten for this node, or `None` when it does not match it.
    pub(super) fn rewrite(&self, selector: &str, node: &'a Node) -> Option<String> {
        let mut steps = split(selector).into_iter().rev();
        let (mut relation, subject) = steps.next()?;
        if !matches_node(subject, node) {
            return None;
        }
        let mut emitted = format!(".{}", self.class(node)?);
        let mut current = node;
        for (combinator, compound) in steps {
            let relationship = relation?;
            let matched = self.relative(current, relationship, compound)?;
            emitted = format!(".{}{relationship}{emitted}", self.class(matched)?);
            relation = combinator;
            current = matched;
        }
        Some(emitted)
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

/// The selector split into compounds, each paired with the combinator that precedes it.
///
/// The leading compound has no combinator. A descendant relationship is reported as a space
/// so every relationship is one character and the walk needs no separate case for it.
fn split(selector: &str) -> Vec<(Option<char>, &str)> {
    let mut steps = Vec::new();
    let mut combinator = None;
    let mut start = None;
    let mut depth = 0usize;
    let mut quote = None;
    for (offset, character) in selector.char_indices() {
        match (quote, character) {
            (Some(open), _) if character == open => quote = None,
            (Some(_), _) => continue,
            (None, '"' | '\'') => quote = Some(character),
            (None, '(' | '[') => depth += 1,
            (None, ')' | ']') => depth = depth.saturating_sub(1),
            (None, _) if depth == 0 && is_combinator(character) => {
                if let Some(begin) = start.take() {
                    steps.push((combinator.take(), &selector[begin..offset]));
                    combinator = Some(' ');
                }
                if character != ' ' {
                    combinator = Some(character);
                }
                continue;
            }
            _ => {}
        }
        start.get_or_insert(offset);
    }
    if let Some(begin) = start {
        steps.push((combinator, selector[begin..].trim_end()));
    }
    steps
}

fn is_combinator(character: char) -> bool {
    character.is_whitespace() || matches!(character, '>' | '+' | '~')
}
