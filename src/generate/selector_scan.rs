//! One reader for CSS selector text.
//!
//! A selector is grammar interleaved with data: a quoted attribute value is a string token
//! and may hold any character the grammar itself uses. Selectors 4 permits `.`, `#`, `]`,
//! `,` and `:` inside one unescaped, precisely because the quotes delimit the value, so
//! every question of the form "where is the next X" is really "where is the next X the
//! grammar owns". That is a single question, and answering it in several places is how
//! readers of one language come to disagree with each other while each looks right alone.

/// Every character of a selector that is not inside a quoted string, paired with its byte
/// offset and the nesting depth it sits at.
///
/// Depth counts `(` and `[` alike, so a caller asking about the top level is answered the
/// same way whether the nesting came from a functional pseudo-class or an attribute
/// selector. The depth reported for an opener is the depth outside it, so in `:is(.a)` the
/// colon, the name and both parens sit at depth 0 while `.a` sits at depth 1.
pub(super) fn unquoted(selector: &str) -> impl Iterator<Item = (usize, char, usize)> + '_ {
    let mut depth = 0usize;
    let mut quote = None;
    selector
        .char_indices()
        .filter_map(move |(offset, character)| match (quote, character) {
            (Some(open), _) if character == open => {
                quote = None;
                None
            }
            (Some(_), _) => None,
            (None, '"' | '\'') => {
                quote = Some(character);
                None
            }
            (None, '(' | '[') => {
                depth += 1;
                Some((offset, character, depth - 1))
            }
            (None, ')' | ']') => {
                depth = depth.saturating_sub(1);
                Some((offset, character, depth))
            }
            (None, _) => Some((offset, character, depth)),
        })
}

/// The offset of the first `wanted` that belongs to the selector's own grammar, as opposed
/// to one a quoted value merely spells out.
///
/// Nesting is deliberately not consulted. A caller that also wants the top level filters
/// [`unquoted`] itself, because for most of these questions depth is the wrong test: the
/// class in `:not(.legacy)` is nested and is still read, and the refusal that keeps such a
/// member out of the generated sheet is made elsewhere, on the member as a whole.
pub(super) fn grammatical(selector: &str, wanted: char) -> Option<usize> {
    unquoted(selector)
        .find(|(_, character, _)| *character == wanted)
        .map(|(offset, _, _)| offset)
}

/// An attribute selector's value with its delimiters removed.
///
/// Exactly one pair is stripped. Trimming every leading and trailing quote instead reads
/// `[lang="'en'"]` as `en`, which is a different value and a different set of elements —
/// the same mistake as reading a quoted `]` as the end of the selector, one layer in.
pub(super) fn unquote_value(value: &str) -> &str {
    let value = value.trim();
    let mut characters = value.chars();
    match (characters.next(), characters.next_back()) {
        (Some(open @ ('"' | '\'')), Some(close)) if open == close => {
            &value[open.len_utf8()..value.len() - close.len_utf8()]
        }
        _ => value,
    }
}

/// The run of characters that spells one identifier, starting at the beginning of `text`.
///
/// A class, an id and a tag are all read this way, so the set of characters that continues
/// a name is stated once. It is deliberately narrower than the CSS ident grammar, which
/// admits escapes and non-ASCII: a name the generator cannot reproduce verbatim is one it
/// must not claim to have matched.
pub(super) fn name(text: &str) -> &str {
    let length = text
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .map(char::len_utf8)
        .sum();
    &text[..length]
}

#[cfg(test)]
#[path = "selector_scan_tests.rs"]
mod selector_scan_tests;
