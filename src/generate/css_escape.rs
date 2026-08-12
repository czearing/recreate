use std::borrow::Cow;

/// How much of an escape sequence is still outstanding.
///
/// CSS Syntax 3 gives `\` one meaning wherever it appears: it denies the next code point its
/// usual role. That is a property of the code-point stream, not of any one construct, so the
/// rule is stated once here and consulted by every reader that walks CSS text.
pub(super) enum Escape {
    None,
    Opened,
    Hex(u8),
}

impl Escape {
    /// Whether `character` is spelled by the escape in progress, advancing it either way.
    ///
    /// A hex escape ends after six digits, or at the first character that is not one. When
    /// that character is whitespace it terminates the escape and is consumed with it;
    /// anything else ends the escape without belonging to it and is answered normally.
    pub(super) fn consumes(&mut self, character: char) -> bool {
        let digits = match std::mem::replace(self, Escape::None) {
            Escape::None => return false,
            Escape::Opened if !character.is_ascii_hexdigit() => return true,
            Escape::Opened => 1,
            Escape::Hex(digits) if digits < 6 && character.is_ascii_hexdigit() => digits + 1,
            Escape::Hex(_) => return character.is_whitespace(),
        };
        *self = Escape::Hex(digits);
        true
    }
}

/// The code points an identifier spells, with its escapes resolved.
///
/// An escape is spelling, not content. A page whose markup says `class="md:flex"` must
/// write that class as `.md\:flex`, because an unescaped colon would open a pseudo-class;
/// the two name the same class. A generator that compares the spelling against the class an
/// element carries therefore demands a class no element has, and drops the rule whole.
///
/// The hex form exists because some code points cannot be written literally at all. An ident
/// may not begin with a digit, so `2xl:flex` can only be spelled `\32 xl\:flex`, and the
/// space there terminates the hex digits rather than separating anything.
///
/// A hex escape naming no character — zero, a surrogate, or a value past the last code point
/// — resolves to the replacement character, which is what CSS Syntax 3 requires and what
/// keeps a name that cannot exist from silently becoming a shorter name that can.
pub(super) fn unescape(spelling: &str) -> Cow<'_, str> {
    if !spelling.contains('\\') {
        return Cow::Borrowed(spelling);
    }
    let mut value = String::with_capacity(spelling.len());
    let mut characters = spelling.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '\\' {
            value.push(character);
            continue;
        }
        let mut hex = String::new();
        while hex.len() < 6 && characters.peek().is_some_and(char::is_ascii_hexdigit) {
            hex.extend(characters.next());
        }
        if hex.is_empty() {
            value.extend(characters.next());
            continue;
        }
        if characters
            .peek()
            .is_some_and(|following| following.is_whitespace())
        {
            characters.next();
        }
        let point = u32::from_str_radix(&hex, 16)
            .ok()
            .filter(|point| *point != 0)
            .and_then(char::from_u32);
        value.push(point.unwrap_or(char::REPLACEMENT_CHARACTER));
    }
    Cow::Owned(value)
}

#[cfg(test)]
#[path = "css_escape_tests.rs"]
mod css_escape_tests;
