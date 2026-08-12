//! What one compound selector means.
//!
//! A compound is the part of a selector that describes a single element — its tag, id,
//! classes and attributes — with no relationship to any other element. Every path that
//! reads an authored selector resolves it through here, so a compound means the same thing
//! whether it names the subject of a rule or an ancestor of it.

use super::selector_list;
use crate::model::Node;

pub(super) fn directly_targets_node(selectors: &str, node: &Node) -> bool {
    selector_list::members(selectors)
        .any(|selector| terminal_compound(selector) == selector && matches_node(selector, node))
}

pub(super) fn matches_node(compound: &str, node: &Node) -> bool {
    let classes = node
        .attributes
        .get("class")
        .into_iter()
        .flat_map(|value| value.split_whitespace())
        .collect::<std::collections::HashSet<_>>();
    let required = compound_classes(compound);
    let tag = compound_tag(compound);
    let id = compound_id(compound);
    let attributes = compound_attributes(compound);
    let constrained =
        !required.is_empty() || !tag.is_empty() || id.is_some() || !attributes.is_empty();
    constrained
        && (tag.is_empty() || tag == "*" || tag == node.tag)
        && id.is_none_or(|id| node.attributes.get("id").is_some_and(|value| value == id))
        && attributes.iter().all(|(name, expected)| {
            node.attributes
                .get(*name)
                .is_some_and(|actual| expected.is_none_or(|expected| actual == expected))
        })
        && required
            .iter()
            .all(|class| classes.contains(class.as_str()))
}

pub(super) fn terminal_compound(selector: &str) -> &str {
    selector
        .trim()
        .rsplit(|character: char| character.is_whitespace() || matches!(character, '>' | '+' | '~'))
        .find(|part| !part.is_empty())
        .unwrap_or_default()
}

pub(super) fn compound_tag(compound: &str) -> &str {
    let length = compound
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '*'))
        .map(char::len_utf8)
        .sum();
    &compound[..length]
}

pub(super) fn compound_classes(compound: &str) -> Vec<String> {
    let mut classes = Vec::new();
    let mut remaining = compound;
    while let Some(index) = remaining.find('.') {
        remaining = &remaining[index + 1..];
        let length = remaining
            .chars()
            .take_while(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            })
            .map(char::len_utf8)
            .sum();
        if length == 0 {
            break;
        }

        classes.push(remaining[..length].to_string());
        remaining = &remaining[length..];
    }

    classes
}

pub(super) fn compound_id(compound: &str) -> Option<&str> {
    let remaining = compound.split_once('#')?.1;
    let length = remaining
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .map(char::len_utf8)
        .sum();
    (length > 0).then_some(&remaining[..length])
}

pub(super) fn compound_attributes(compound: &str) -> Vec<(&str, Option<&str>)> {
    let mut attributes = Vec::new();
    let mut remaining = compound;
    while let Some((_, after_open)) = remaining.split_once('[') {
        let Some((attribute, after_close)) = after_open.split_once(']') else {
            break;
        };
        let (name, value) = attribute
            .split_once('=')
            .map_or((attribute, None), |(name, value)| {
                (
                    name,
                    Some(
                        value
                            .trim()
                            .trim_matches(|character| matches!(character, '"' | '\'')),
                    ),
                )
            });
        let name = name.trim();
        if !name.is_empty() {
            attributes.push((name, value));
        }
        remaining = after_close;
    }
    attributes
}
