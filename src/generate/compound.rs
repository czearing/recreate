//! What one compound selector means.
//!
//! A compound is the part of a selector that describes a single element — its tag, id,
//! classes and attributes — with no relationship to any other element. Every path that
//! reads an authored selector resolves it through here, so a compound means the same thing
//! whether it names the subject of a rule or an ancestor of it.

use std::borrow::Cow;

use super::css_escape::unescape;
use super::css_scan::{grammatical, name, unquote_value, unquoted};
use super::selector_list;
use crate::model::Node;

pub(super) fn directly_targets_node(selectors: &str, node: &Node) -> bool {
    Subject::new(node).directly_targeted_by(selectors)
}

/// A node reduced to what a subject compound is allowed to ask about, gathered once.
///
/// Matching one node against a page's rules asks these same questions once per candidate
/// rule, and the class set is the costly answer — read from the attribute each time, it is
/// rebuilt thousands of times per node for an answer that cannot change.
pub(super) struct Subject<'n> {
    node: &'n Node,
    classes: std::collections::HashSet<&'n str>,
}

impl<'n> Subject<'n> {
    pub(super) fn new(node: &'n Node) -> Self {
        Self {
            node,
            classes: node
                .attributes
                .get("class")
                .into_iter()
                .flat_map(|value| value.split_whitespace())
                .collect(),
        }
    }

    pub(super) fn directly_targeted_by(&self, selectors: &str) -> bool {
        selector_list::members(selectors)
            .any(|selector| terminal_compound(selector) == selector && self.matches(selector))
    }

    pub(super) fn matches(&self, compound: &str) -> bool {
        let required = compound_classes(compound);
        let tag = compound_tag(compound);
        let id = compound_id(compound);
        let attributes = compound_attributes(compound);
        let constrained =
            !required.is_empty() || !tag.is_empty() || id.is_some() || !attributes.is_empty();
        constrained
            && (tag.is_empty() || tag == "*" || tag == self.node.tag)
            && id.is_none_or(|id| {
                self.node
                    .attributes
                    .get("id")
                    .is_some_and(|value| value.as_str() == id)
            })
            && attributes.iter().all(|(name, expected)| {
                self.node
                    .attributes
                    .get(*name)
                    .is_some_and(|actual| expected.is_none_or(|expected| actual == expected))
            })
            && required
                .iter()
                .all(|class| self.classes.contains(class.as_str()))
    }
}

pub(super) fn matches_node(compound: &str, node: &Node) -> bool {
    Subject::new(node).matches(compound)
}

pub(super) fn terminal_compound(selector: &str) -> &str {
    selector
        .trim()
        .rsplit(is_combinator)
        .find(|part| !part.is_empty())
        .unwrap_or_default()
}

/// The type selector a compound names, or the universal selector, or nothing.
///
/// Kept apart from [`name`] because these are two lexical classes, not one: a type
/// selector may be `*`, and a class or id may not.
pub(super) fn compound_tag(compound: &str) -> &str {
    if compound.starts_with('*') {
        return &compound[..1];
    }
    name(compound)
}

/// The classes a compound requires an element to carry.
///
/// Only a dot the selector's own grammar owns opens a class. A dot inside a quoted
/// attribute value is data — `a[href="https://example.com/"]` requires no class at all —
/// and reading one as a class adds a requirement no element meets, which drops the rule
/// whole rather than mismatching it.
pub(super) fn compound_classes(compound: &str) -> Vec<String> {
    let mut classes = Vec::new();
    let mut remaining = compound;
    while let Some(index) = grammatical(remaining, '.') {
        remaining = &remaining[index + 1..];
        let class = name(remaining);
        if class.is_empty() {
            break;
        }

        classes.push(unescape(class).into_owned());
        remaining = &remaining[class.len()..];
    }

    classes
}

/// The id a compound requires, if it names one.
///
/// A fragment is the ordinary counter-example: `a[href="#main"]` names no id, and reading
/// its hash as one demands `id="main"` on the link rather than on its destination, which no
/// in-page link carries.
pub(super) fn compound_id(compound: &str) -> Option<Cow<'_, str>> {
    let index = grammatical(compound, '#')?;
    let id = name(&compound[index + 1..]);
    (!id.is_empty()).then(|| unescape(id))
}

/// The attributes a compound requires, each with the exact value it demands when it names
/// one.
///
/// The bracket that closes an attribute selector is the first one outside the value's
/// quotes. Cutting at the first bracket of any kind truncates `[data-token="a]b"]` to the
/// value `a`, and unlike the class and id cases that failure over-matches: the rule is
/// emitted, onto the elements the author excluded.
pub(super) fn compound_attributes(compound: &str) -> Vec<(&str, Option<&str>)> {
    let mut attributes = Vec::new();
    let mut remaining = compound;
    while let Some(open) = grammatical(remaining, '[') {
        let after_open = &remaining[open + 1..];
        let Some(close) = grammatical(after_open, ']') else {
            break;
        };
        let attribute = &after_open[..close];
        let (name, value) = attribute
            .split_once('=')
            .map_or((attribute, None), |(name, value)| {
                (name, Some(unquote_value(value)))
            });
        let name = name.trim();
        if !name.is_empty() {
            attributes.push((name, value));
        }
        remaining = &after_open[close + 1..];
    }
    attributes
}

/// The selector split into compounds, each paired with the combinator that precedes it.
///
/// The leading compound has no combinator. A descendant relationship is reported as a space
/// so every relationship is one character and the walk needs no separate case for it.
pub(super) fn split(selector: &str) -> Vec<(Option<char>, &str)> {
    let mut steps = Vec::new();
    let mut combinator = None;
    let mut start = None;
    for (offset, character, depth) in unquoted(selector) {
        if depth == 0 && is_combinator(character) {
            if let Some(begin) = start.take() {
                steps.push((combinator.take(), &selector[begin..offset]));
                combinator = Some(' ');
            }
            if character != ' ' {
                combinator = Some(character);
            }
            continue;
        }
        start.get_or_insert(offset);
    }
    if let Some(begin) = start {
        steps.push((combinator, selector[begin..].trim_end()));
    }
    steps
}

pub(super) fn is_combinator(character: char) -> bool {
    character.is_whitespace() || matches!(character, '>' | '+' | '~')
}
