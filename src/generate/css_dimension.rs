//! The dimension tokens a CSS value contains.
//!
//! [`super::css_scan`] answers where the next character the grammar owns is. That is a
//! question about structure, and it is asked of selectors, blocks and values alike. This
//! asks a different one — which tokens a value is built from — and only a value has it. The
//! two share the scanner because a token cannot begin inside a quoted string, but they are
//! not the same question and a caller wanting one never wants the other.

use super::css_scan::{identifier, name, unquoted};

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

#[cfg(test)]
#[path = "css_dimension_tests.rs"]
mod css_dimension_tests;
