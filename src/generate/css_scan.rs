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

/// Every character of CSS text that is not inside a quoted string, paired with its byte
/// offset and the nesting depth it sits at.
///
/// Depth counts `(`, `[` and `{` alike, so a caller asking about the top level is answered
/// the same way whether the nesting came from a functional pseudo-class, an attribute
/// selector or a nested block. The depth reported for an opener is the depth outside it, so
/// in `:is(.a)` the colon, the name and both parens sit at depth 0 while `.a` sits at
/// depth 1; a closer is likewise reported at the depth outside the block it ends.
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
            (None, '(' | '[' | '{') => {
                depth += 1;
                Some((offset, character, depth - 1))
            }
            (None, ')' | ']' | '}') => {
                depth = depth.saturating_sub(1);
                Some((offset, character, depth))
            }
            (None, _) => Some((offset, character, depth)),
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

/// The unit of every dimension and percentage token in `text`, with quoted regions skipped.
///
/// A unit only means a unit as part of a token. `<dimension-token>` is a number immediately
/// followed by an identifier, so `30vw` carries a unit while `--vwrap`, a font named `Vwide`
/// and a `url(a%20b.png)` path merely spell the letters. Reading the number first is the only
/// thing that separates them, and is why a caller can ask about a unit here without the
/// over-matching that stops a substring scan from ever naming one.
///
/// A number is only a number where an identifier is not already running: the `2` in
/// `Roboto2Wide` continues an identifier and starts nothing, and a digit already consumed as
/// part of an earlier number cannot start a second one. Adjacency is decided by byte offset,
/// so a quoted region breaks a token rather than joining its neighbours.
pub(super) fn units(text: &str) -> Vec<&str> {
    let mut units = Vec::new();
    let mut previous: Option<(usize, char)> = None;
    let mut consumed = 0usize;
    for (offset, character, _) in unquoted(text) {
        let joined = previous.is_some_and(|(at, last)| at + last.len_utf8() == offset);
        let continues = previous.is_some_and(|(_, last)| identifier(last));
        if character.is_ascii_digit() && offset >= consumed && !(joined && continues) {
            let rest = number(&text[offset..]);
            let unit = if rest.starts_with('%') {
                "%"
            } else {
                name(rest)
            };
            consumed = text.len() - rest.len() + unit.len();
            if !unit.is_empty() {
                units.push(unit);
            }
        }
        previous = Some((offset, character));
    }
    units
}

/// The text following the numeric token that begins `text`.
///
/// A fractional part needs no handling of its own: its digits begin a numeric token in their
/// own right, carrying the same unit, so reading `12.5vw` as one number or as two yields the
/// same unit either way. An exponent does need it — without it `1e3vw` reads its unit as
/// `e3vw`, which is a unit the value never spelled.
fn number(text: &str) -> &str {
    let rest = digits(text);
    rest.strip_prefix(['e', 'E'])
        .map(|rest| rest.strip_prefix(['+', '-']).unwrap_or(rest))
        .filter(|rest| rest.starts_with(|character: char| character.is_ascii_digit()))
        .map_or(rest, digits)
}

/// The text following a run of decimal digits.
fn digits(text: &str) -> &str {
    text.trim_start_matches(|character: char| character.is_ascii_digit())
}

/// Whether a character continues an identifier that has already started.
fn identifier(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
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
        .take_while(|character| identifier(*character))
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
#[path = "css_scan_unit_tests.rs"]
mod css_scan_unit_tests;
