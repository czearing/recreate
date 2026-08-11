//! The names a stylesheet's text spells: which custom properties it reads, what value a
//! rule gives one, and whether a given identifier occurs at all.
//!
//! Each is a question about text rather than about style, because the values carrying
//! them survive into computed style unresolved — `var()` is substituted at use, an
//! animation name is reported as a bare token. Parsing a value as a value answers none of
//! them. One owner also keeps the identifier boundary single: split across callers, `spin`
//! gets read out of `arcspin` in one place and not in another.

use std::collections::BTreeSet;

/// The custom properties `text` reads through `var()`.
pub(super) fn references(text: &str) -> BTreeSet<String> {
    let mut references = BTreeSet::new();
    let mut remaining = text;
    while let Some(index) = remaining.find("var(--") {
        remaining = &remaining[index + 4..];
        let end = remaining
            .find([',', ')', ' ', '\t'])
            .unwrap_or(remaining.len());
        references.insert(remaining[..end].to_string());
        remaining = &remaining[end..];
    }
    references
}

/// The value `text` declares for the custom property `name`, if it declares one.
///
/// The value is returned as written, empty included, because whether an empty value counts
/// is the caller's question: to one it is a declaration that makes every reader invalid,
/// to another it is no declaration at all.
pub(super) fn declared_value(text: &str, name: &str) -> Option<String> {
    let mut from = 0;
    while let Some(relative) = mention_index(&text[from..], name) {
        let start = from + relative + name.len();
        if let Some(value) = text[start..].trim_start().strip_prefix(':') {
            let end = value.find([';', '}']).unwrap_or(value.len());
            return Some(value[..end].trim().to_string());
        }
        from = start;
    }
    None
}

/// Whether `name` occurs as a whole identifier rather than inside a longer one, so that
/// `spin` is not read out of `arcspin`.
pub(super) fn mentions(text: &str, name: &str) -> bool {
    mention_index(text, name).is_some()
}

pub(super) fn mention_index(text: &str, name: &str) -> Option<usize> {
    if name.is_empty() {
        return None;
    }
    let mut from = 0;
    while let Some(relative) = text[from..].find(name) {
        let start = from + relative;
        let end = start + name.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        if !before.is_some_and(identifier_character) && !after.is_some_and(identifier_character) {
            return Some(start);
        }
        from = end;
    }
    None
}

fn identifier_character(character: char) -> bool {
    character.is_alphanumeric() || character == '-' || character == '_'
}
