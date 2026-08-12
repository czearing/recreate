use super::super::compound::{
    compound_attributes, compound_classes, compound_id, compound_tag, split,
};
use super::{grammatical, name, unquote_value};

/// The defect this module was extracted to end. Every character below is legal inside a
/// quoted attribute value per Selectors 4, and each is also punctuation the selector
/// grammar uses, so a scanner that reads bytes rather than grammar finds all of them.
#[test]
fn punctuation_inside_a_quoted_value_belongs_to_the_value() {
    let selector = r#"a[href="https://example.com/#main,x:y"]"#;
    assert_eq!(grammatical(selector, '.'), None);
    assert_eq!(grammatical(selector, '#'), None);
    assert_eq!(grammatical(selector, ','), None);
    assert_eq!(grammatical(selector, ':'), None);
    // The bracket that closes the selector is the one after the closing quote, not the
    // first one the bytes contain.
    assert_eq!(grammatical(selector, ']'), Some(selector.len() - 1));
}

/// The same punctuation outside quotes is grammar and must still be found. A scanner that
/// skipped too much would silently stop recognising classes altogether.
#[test]
fn the_same_punctuation_outside_quotes_is_still_grammar() {
    let selector = "div.card#lead[data-x]";
    assert_eq!(grammatical(selector, '.'), Some(3));
    assert_eq!(grammatical(selector, '#'), Some(8));
    assert_eq!(grammatical(selector, '['), Some(13));
    assert_eq!(grammatical(selector, ']'), Some(selector.len() - 1));
}

/// Nesting is not consulted by `grammatical`, and that is load-bearing rather than an
/// oversight. `compound_classes` reads `.legacy` here, which is wrong, and it is harmless
/// only because the member is refused upstream for its colon. Teaching this scanner to
/// skip nested regions would silently arm the complement bug the refusal exists to
/// prevent, so the reading is pinned as it stands.
#[test]
fn nesting_is_not_a_reason_to_skip_a_character() {
    assert_eq!(grammatical(".theme:not(.legacy)", '.'), Some(0));
    assert_eq!(compound_classes(".theme:not(.legacy)"), ["theme", "legacy"]);
}

/// A value carries exactly one pair of delimiters. Trimming every quote at either end
/// reads `'en'` as `en`, which selects a different set of elements.
#[test]
fn exactly_one_pair_of_delimiters_is_stripped() {
    assert_eq!(unquote_value(r#""en""#), "en");
    assert_eq!(unquote_value(r#" 'en' "#), "en");
    assert_eq!(unquote_value(r#""'en'""#), "'en'");
    assert_eq!(unquote_value("en"), "en");
    assert_eq!(unquote_value(r#"""#), r#"""#);
    assert_eq!(unquote_value(""), "");
}

/// A name ends where the grammar's next token begins, and the run is the same one for a
/// class and for an id.
#[test]
fn a_name_stops_at_the_next_token() {
    assert_eq!(name("card-lead.other"), "card-lead");
    assert_eq!(name("lead[data-x]"), "lead");
    assert_eq!(name("_private"), "_private");
    assert_eq!(name(".immediate"), "");
}

/// A dot inside a quoted value is data. Reading it as a class adds a requirement no
/// element carries, and the rule is then dropped whole — no selector, no declaration.
#[test]
fn a_quoted_dot_does_not_invent_a_class() {
    assert!(compound_classes(r#"a[href="https://example.com/"]"#).is_empty());
    assert_eq!(
        compound_classes(r#"a.card[href="https://example.com/"]"#),
        ["card"]
    );
}

/// A fragment is not an id. `a[href="#main"]` says nothing about the anchor's own id, and
/// demanding one there is a requirement no in-page link meets.
#[test]
fn a_quoted_hash_does_not_invent_an_id() {
    assert_eq!(compound_id(r##"a[href="#main"]"##), None);
    assert_eq!(compound_id(r##"a#lead[href="#main"]"##), Some("lead"));
}

/// The bracket that closes an attribute selector is the first one outside the value's
/// quotes. Cutting at the first bracket of any kind truncates the value, and unlike the
/// class and id cases that failure OVER-matches: the rule is emitted onto the elements the
/// author excluded, so the output is wrong rather than merely incomplete.
#[test]
fn a_quoted_bracket_does_not_end_the_attribute_selector() {
    assert_eq!(
        compound_attributes(r#"a[data-token="a]b"]"#),
        [("data-token", Some("a]b"))]
    );
    assert_ne!(
        compound_attributes(r#"a[data-token="a]b"]"#),
        compound_attributes(r#"a[data-token="a"]"#)
    );
}

/// Several attribute selectors in one compound, the second reached only by resuming after
/// the true closing bracket. Resuming after a truncated one loses it entirely.
#[test]
fn every_attribute_of_a_compound_is_read_even_after_a_quoted_bracket() {
    assert_eq!(
        compound_attributes(r#"a[data-token="a]b"][href="x"][hidden]"#),
        [
            ("data-token", Some("a]b")),
            ("href", Some("x")),
            ("hidden", None)
        ]
    );
}

/// A type selector is its own lexical class. `*` is a tag and is never a name.
#[test]
fn the_universal_selector_is_a_tag_and_a_name_is_not() {
    assert_eq!(compound_tag("*"), "*");
    assert_eq!(compound_tag("*.card"), "*");
    assert_eq!(compound_tag("div.card"), "div");
    assert_eq!(compound_tag(".card"), "");
    assert_eq!(compound_tag(r#"[href="a.b"]"#), "");
}

/// `split` reads the same string as the scanners above, so it must skip the same regions.
/// A combinator spelled inside a value would otherwise cut one compound into two, and the
/// halves are selectors that match things the whole does not.
#[test]
fn a_combinator_spelled_inside_a_value_does_not_cut_the_compound() {
    for value in ["a b", "a>b", "a+b", "a~b"] {
        let selector = format!(r#"a[data-token="{value}"]"#);
        assert_eq!(split(&selector), [(None, selector.as_str())], "{value}");
    }
    assert_eq!(
        split(r#"nav a[data-token="a b"] > span"#),
        [
            (None, "nav"),
            (Some(' '), r#"a[data-token="a b"]"#),
            (Some('>'), "span")
        ]
    );
}

/// A bracket nests exactly as a paren does, so the whitespace CSS permits around an
/// attribute selector's matcher is inside the brackets and is not a descendant combinator.
/// A scanner that counted only parens would cut this compound into three.
#[test]
fn whitespace_inside_an_attribute_selector_is_not_a_combinator() {
    let selector = r#"a[ data-token = "x" ]"#;
    assert_eq!(split(selector), [(None, selector)]);
    assert_eq!(compound_attributes(selector), [("data-token", Some("x"))]);
}
