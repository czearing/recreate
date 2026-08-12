use super::super::compound::split;
use super::super::css_dimension::units;
use super::{block, grammatical, name, unquote_value};

/// CSS Syntax 3 runs a string to its delimiter, a newline, or EOF, and a `\` consumes the
/// next code point. A quote therefore closes a string only when an even number of
/// backslashes precede it, which is a question a scanner without a backslash arm cannot ask.
///
/// The escape is not exotic authoring. CSSOM serialises a value holding a double quote by
/// escaping it, so `font-family: 'A"B'` reaches this reader as `font-family: "A\"B"` with a
/// backslash the author never typed.
#[test]
fn an_escaped_quote_does_not_close_the_string_it_sits_in() {
    let rule =
        r#"@media (prefers-color-scheme: dark) { .alpha { font-family: "A\"B"; color: #b00 } }"#;
    let (before, inside, after) = block(rule).expect("the media block");

    assert_eq!(before, "@media (prefers-color-scheme: dark) ");
    assert_eq!(inside, r#" .alpha { font-family: "A\"B"; color: #b00 } "#);
    assert_eq!(after, "");
}

/// The opposite direction of the same omission. A class containing a colon must be spelled
/// with an escape, so reading that colon as grammar invents a pseudo-class the selector
/// never had and the rule is refused. One missing arm reads grammar as data in a string and
/// data as grammar in a selector.
#[test]
fn an_escaped_colon_in_a_class_is_not_the_selectors_own_colon() {
    assert_eq!(grammatical(r".md\:flex", ':'), None);
    assert_eq!(grammatical(r".md\:flex:hover", ':'), Some(9));
}

/// An escape is part of the code-point stream, not of the string sublanguage, so it must be
/// honoured outside a string as well as inside. A `.` spelled `\.` belongs to the class name.
#[test]
fn an_escaped_structural_character_outside_a_string_is_content() {
    assert_eq!(grammatical(r".w-1\/2", '/'), None);
    assert_eq!(grammatical(r".p-\[10px\]", '['), None);
    assert_eq!(grammatical(r".a\.b.real", '.'), Some(0));
}

/// A backslash that is itself escaped consumes nothing further, so the quote after it really
/// does close the string. Parity, not the mere presence of a backslash, decides.
#[test]
fn an_escaped_backslash_leaves_the_following_quote_delimiting() {
    let rule = r#".alpha { content: "A\\"; color: red }"#;
    let (_, inside, _) = block(rule).expect("a block");
    assert_eq!(inside, r#" content: "A\\"; color: red "#);

    assert_eq!(grammatical(r#""a\\" . b"#, '.'), Some(6));
}

/// A trailing backslash has nothing to consume, and a scanner that assumed otherwise would
/// index past the end of the text.
#[test]
fn a_backslash_at_the_end_of_the_text_consumes_nothing() {
    assert_eq!(grammatical(r".alpha\", '.'), Some(0));
    assert_eq!(
        block(r".alpha { color: red }\"),
        Some((".alpha ", " color: red ", r"\"))
    );
}

/// A hex escape owns the space that terminates it. An ident cannot begin with a digit, so
/// Tailwind's `2xl:` variant is spelled `.\32 xl\:flex`, and a capture confirms the browser
/// serialises it that way. Handing the terminator on reports a descendant combinator the
/// selector never had, splitting one class into two.
#[test]
fn the_space_ending_a_hex_escape_is_not_a_descendant_combinator() {
    let selector = r".\32 xl\:flex";
    assert_eq!(grammatical(selector, ' '), None);
    assert_eq!(grammatical(selector, ':'), None);
    assert_eq!(split(selector), vec![(None, selector)]);
}

/// A hex escape ends after six digits, and the character beyond that length is ordinary
/// again. `\000041 ` is six digits and its terminator, so the space belongs to it; in
/// `\0000411` the seventh digit ends the escape, leaving the following space a real
/// separator. Without the bound an escape would swallow grammar arbitrarily far away.
#[test]
fn a_hex_escape_stops_owning_characters_after_six_digits() {
    assert_eq!(grammatical(r".a\000041 b .c", ' '), Some(11));
    assert_eq!(grammatical(r".a\0000411 b", ' '), Some(10));
}

/// The backslash is yielded because callers slice by these offsets. A compound that begins
/// with an escape loses its first character if the reader hides it.
#[test]
fn a_compound_beginning_with_an_escape_keeps_that_escape() {
    assert_eq!(
        split(r"div > \.odd"),
        vec![(None, "div"), (Some('>'), r"\.odd")]
    );
}

/// The whole point of the arm is that it changes nothing for text carrying no backslash.
/// Every earlier answer this reader gives must survive untouched.
#[test]
fn text_without_a_backslash_is_answered_exactly_as_before() {
    let selector = "div.card:is(.a, .b)[data-x=\"{\"]";
    assert_eq!(grammatical(selector, '.'), Some(3));
    assert_eq!(grammatical(selector, '{'), None);
    assert_eq!(unquote_value(r#""en""#), "en");
    assert_eq!(units("calc(var(--gutter) + 30vw)"), vec!["vw"]);
    assert_eq!(name("flex-1 more"), "flex-1");
}
