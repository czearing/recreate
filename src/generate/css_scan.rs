//! One reader for CSS text.
//!
//! CSS is grammar interleaved with data: a quoted string may hold any character the grammar
//! itself uses. Selectors 4 permits `.`, `#`, `]`, `,` and `:` inside a quoted attribute
//! value, and a declaration value may just as legally hold `{` or `}`. In both the quotes
//! delimit the data, so every question of the form "where is the next X" is really "where
//! is the next X the grammar owns". That is a single question, and answering it in several
//! places is how readers of one language come to disagree while each looks right alone.
//!
//! The reader is named for the language rather than for a caller. Selector text and a
//! stylesheet body are different languages, but the part relied on here — CSS's string
//! token — is defined once and identically for both, so one reader serves both. Scoping it
//! to the first caller is what leaves the second writing its own.

use super::css_escape::Escape;

/// Every character of CSS text that is not inside a quoted string and is not spelled by an
/// escape, paired with its byte offset and the nesting depth it sits at.
///
/// Depth counts `(`, `[` and `{` alike, so a caller asking about the top level is answered
/// the same way whether the nesting came from a functional pseudo-class, an attribute
/// selector or a nested block. The depth reported for an opener is the depth outside it, so
/// in `:is(.a)` the colon, the name and both parens sit at depth 0 while `.a` sits at
/// depth 1; a closer is likewise reported at the depth outside the block it ends.
///
/// A `\` escapes what follows, so a quote closes a string only when an even number of
/// backslashes precede it. The escape belongs to the code-point stream rather than to the
/// string sublanguage, so it is honoured in both states: inside a string it stops `"A\"B"`
/// from ending early, and outside one it stops the colon of `.md\:flex` from being read as
/// the selector's own.
///
/// An escape may also spell a code point in hex, and then up to six hex digits and one
/// following space belong to it. That space is part of the escape, not a separator: an ident
/// cannot begin with a digit, so the class `2xl:flex` must be written `.\32 xl\:flex`, and a
/// reader that hands the space on reports a descendant combinator that was never written.
///
/// The backslash itself is yielded. It spells no grammar, but callers slice by the offsets
/// reported here, and a compound beginning with an escape loses its first character if the
/// reader hides it. What the escape spells is withheld, since that is the character being
/// denied its usual meaning.
///
/// Not modelled, and inert: `\` before a newline is not a valid escape, and `\` at the end
/// of the text spells nothing. Both concern characters no caller ever seeks.
pub(super) fn unquoted(selector: &str) -> impl Iterator<Item = (usize, char, usize)> + '_ {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escape = Escape::None;
    selector
        .char_indices()
        .filter_map(move |(offset, character)| {
            if escape.consumes(character) {
                return None;
            }
            match (quote, character) {
                (_, '\\') => {
                    escape = Escape::Opened;
                    Some((offset, character, depth))
                }
                (Some(open), _) if character == open => {
                    quote = None;
                    None
                }
                (Some(_), _) => None,
                (None, '"' | '\'') => {
                    quote = Some(character);
                    None
                }
                (None, '(' | '[' | '{') => {
                    depth += 1;
                    Some((offset, character, depth - 1))
                }
                (None, ')' | ']' | '}') => {
                    depth = depth.saturating_sub(1);
                    Some((offset, character, depth))
                }
                (None, _) => Some((offset, character, depth)),
            }
        })
}

/// The text before the first brace-delimited block, the text inside it, and the text after
/// it.
///
/// This is one primitive rather than a split on `{` and another on `}`, because the two
/// braces are one question: a block ends at the closer that matches its own opener. Reading
/// them separately is what lets a `}` inside a quoted value pass for the end of a rule, and
/// what makes a nested block's closer look like the end of the block containing it.
///
/// Depth is why no trailing-brace trimming is needed. A trim strips whatever braces happen
/// to sit at the end of the text, so it cannot tell a block's own closer from one belonging
/// to something nested inside it; the matching closer is found here instead of guessed at.
///
/// Only the closer consults depth. The first unquoted `{` opens the block whatever depth is
/// reported for it, because reaching one at all means no block is open yet, and CSS gives a
/// brace no meaning inside the `(` or `[` that could have raised the depth.
pub(super) fn block(text: &str) -> Option<(&str, &str, &str)> {
    let mut open = None;
    for (offset, character, depth) in unquoted(text) {
        match (character, depth, open) {
            ('{', _, None) => open = Some(offset),
            ('}', 0, Some(start)) => {
                return Some((
                    &text[..start],
                    &text[start + 1..offset],
                    &text[offset + 1..],
                ));
            }
            _ => {}
        }
    }
    None
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

/// Whether a character continues an identifier that has already started.
pub(super) fn identifier(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
}

/// The run of characters that spells one identifier, starting at the beginning of `text`.
///
/// A class, an id and a tag are all read this way, so the set of characters that continues
/// a name is stated once. Escapes are part of the run: `.md\:flex` names one class, and a
/// reader that stopped at the backslash would report the class `md`, which no element
/// carries. The run is the name's spelling; [`css_escape::unescape`] turns it into the name.
pub(super) fn name(text: &str) -> &str {
    let mut escape = Escape::None;
    let length = text
        .chars()
        .take_while(|character| {
            escape.consumes(*character) || {
                if *character == '\\' {
                    escape = Escape::Opened;
                }
                *character == '\\' || identifier(*character)
            }
        })
        .map(char::len_utf8)
        .sum();
    &text[..length]
}

#[cfg(test)]
#[path = "css_scan_tests.rs"]
mod css_scan_tests;

#[cfg(test)]
#[path = "css_scan_block_tests.rs"]
mod css_scan_block_tests;

#[cfg(test)]
#[path = "css_scan_escape_tests.rs"]
mod css_scan_escape_tests;
