use super::unescape;

/// An escape is spelling, not content. `.md\:flex` and `class="md:flex"` name one class, and
/// a generator that keeps the backslash demands a class the page does not have.
#[test]
fn an_escaped_structural_character_resolves_to_the_character_itself() {
    assert_eq!(unescape(r"md\:flex"), "md:flex");
    assert_eq!(unescape(r"w-1\/2"), "w-1/2");
    assert_eq!(unescape(r"p-\[10px\]"), "p-[10px]");
}

/// A CSS ident may not begin with a digit, so a leading digit can only be spelled in hex,
/// and the space that ends the digits belongs to the escape rather than to the name.
#[test]
fn a_hex_escape_resolves_to_its_code_point_and_swallows_its_terminator() {
    assert_eq!(unescape(r"\32 xl\:flex"), "2xl:flex");
    assert_eq!(unescape(r"\31 0"), "10");
}

/// Six digits is the limit. A seventh is an ordinary character of the name, so a reader
/// without the bound would fold it into the code point and produce a different name.
#[test]
fn a_hex_escape_ends_after_six_digits() {
    assert_eq!(unescape(r"\0000411"), "A1");
    assert_eq!(unescape(r"\41x"), "Ax");
}

/// A hex escape naming no character resolves to the replacement character. Dropping it
/// instead would silently rename `a\0 b` to the real, different class `ab`.
#[test]
fn a_hex_escape_naming_no_character_resolves_to_the_replacement_character() {
    assert_eq!(unescape(r"a\0 b"), "a\u{fffd}b");
    assert_eq!(unescape(r"a\d800 b"), "a\u{fffd}b");
    assert_eq!(unescape(r"a\110000 b"), "a\u{fffd}b");
}

/// A backslash escaping a backslash spells one backslash, and leaves what follows ordinary.
#[test]
fn an_escaped_backslash_spells_one_backslash() {
    assert_eq!(unescape(r"a\\3 b"), r"a\3 b");
}

/// A name carrying no backslash is returned untouched, without allocating, so every page
/// that spells no escape is answered exactly as before.
#[test]
fn a_name_without_an_escape_is_borrowed_unchanged() {
    let borrowed = unescape("md-flex");
    assert_eq!(borrowed, "md-flex");
    assert!(matches!(borrowed, std::borrow::Cow::Borrowed(_)));
}
