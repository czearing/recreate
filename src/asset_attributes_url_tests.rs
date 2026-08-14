//! What terminates a CSS `url()` value is decided by what opened it, not by a character.
//!
//! CSSOM never hands back authored text. It serialises a URL as `url(` + *serialize a
//! string* + `)`, and serialize-a-string wraps the value in `"` and escapes exactly two
//! code points: `"` and `\`. A `)` or a `'` inside the URL is therefore emitted raw,
//! inside the quotes, where it is string content and not structure.
//!
//! CSS Syntax gives the reason one character class cannot serve. `url(` followed by a
//! quote is a *function-token* whose argument is a *string-token*, ending only at the
//! matching unescaped quote; `url(` followed by anything else is a *url-token*, ending at
//! the first unescaped `)`. Two productions, two different terminators — so a class that
//! admits `)` cannot terminate a url-token, and one that admits `'` cannot terminate a
//! single-quoted string. Holding both below is what makes widening a class fail
//! categorically rather than numerically.
//!
//! Collecting is what decides which bytes the artifact contains. A URL missed here is
//! never fetched, never keyed, and survives into the emitted CSS as a live address on the
//! capture rig's ephemeral port — so the recreation repaints only while the rig is up.

use crate::node_eval;
use serde_json::{Value, json};

const HARNESS: &str = include_str!("asset_attributes_reach_harness.js");
const BASE: &str = "http://rig.test:59700/page.html";
const ORIGIN: &str = "http://rig.test:59700/";

/// Everything `recreateAssetUrls` collected from one stylesheet rule, sorted.
fn collected(rules: Value) -> Vec<String> {
    let tree = json!({ "tag": "div" });
    let result = node_eval::json(
        &HARNESS
            .replace("__ASSET_ATTRIBUTES__", &super::js_source())
            .replace("__TREE__", &tree.to_string())
            .replace("__CSS_RULES__", &rules.to_string())
            .replace("__BASE__", BASE),
    );
    result["assets"]
        .as_array()
        .expect("assets")
        .iter()
        .map(|value| value.as_str().unwrap_or_default().to_string())
        .collect()
}

/// What one declaration's `url()` collected to.
fn from_rule(value: &str) -> Vec<String> {
    collected(json!([format!("#a {{ background-image: {value}; }}")]))
}

fn url(path: &str) -> String {
    format!("{ORIGIN}{path}")
}

/// Anti-vacuity. A value with no delimiter character in it must collect, or every absence
/// below has an innocent cause and proves nothing.
#[test]
fn collects_a_double_quoted_value() {
    assert_eq!(from_rule(r#"url("plain.png")"#), vec![url("plain.png")]);
}

/// The truncating arm. A raw `)` inside the quotes is string content; the value ends at
/// the quote that opened it, not at the first `)`. A prefix collected here is worse than
/// nothing: it is fetched, 404s silently, and never becomes a key.
#[test]
fn collects_a_closing_paren_inside_a_quoted_value() {
    assert_eq!(
        from_rule(r#"url("chart(1).png")"#),
        vec![url("chart(1).png")]
    );
}

/// The vanishing arm. A raw `'` inside double quotes is likewise content, and it fails
/// differently: the value is not shortened, it is never seen.
#[test]
fn collects_a_single_quote_inside_a_double_quoted_value() {
    assert_eq!(from_rule(r#"url("it's.png")"#), vec![url("it's.png")]);
}

/// The mirror: a `"` inside single quotes. Whichever quote opened the value is the only
/// one that can close it, so neither delimiter is privileged.
#[test]
fn collects_a_double_quote_inside_a_single_quoted_value() {
    assert_eq!(from_rule("url('say\\\"hi.png')"), vec![url("say%22hi.png")]);
}

/// The other production. Unquoted, a `)` *is* the terminator — the case any class widened
/// to admit `)` would break, which is why the two arms cannot be satisfied at once by a
/// wider class.
#[test]
fn collects_an_unquoted_value() {
    assert_eq!(from_rule("url(bare.png)"), vec![url("bare.png")]);
}

/// Single quotes are an equal alternative to double, not a fallback.
#[test]
fn collects_a_single_quoted_value() {
    assert_eq!(from_rule("url('single.png')"), vec![url("single.png")]);
}

/// Whitespace is allowed on both sides of the value inside `url( … )`, and it is not part
/// of the URL. A reader that includes it keys the map on a string the CSS text never
/// contains.
#[test]
fn ignores_whitespace_around_a_quoted_value() {
    assert_eq!(
        from_rule(r#"url(  "spaced.png"  )"#),
        vec![url("spaced.png")]
    );
}

/// A backslash escape is structure in *both* productions, so an escaped `)` inside an
/// unquoted value does not end it.
#[test]
fn honours_an_escape_in_an_unquoted_value() {
    assert_eq!(from_rule("url(a\\).png)"), vec![url("a).png")]);
}

/// Several values in one declaration are several assets. A reader that gives up after the
/// first, or that runs one value into the next, loses the rest of the layer list.
#[test]
fn collects_every_value_in_one_declaration() {
    assert_eq!(
        from_rule(r#"url("a(1).png"), url('b.png'), url(c.png)"#),
        vec![url("a(1).png"), url("b.png"), url("c.png")]
    );
}

/// A data URI must still be *matched* — only the download pass excludes it, because its
/// bytes are already inline. Excluding it from matching instead would leave the reader
/// resynchronising in the middle of a value whose payload can contain anything.
#[test]
fn matches_a_data_uri_without_collecting_it() {
    assert!(
        from_rule(
            "url(\"data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg'/>\"), url(after.png)"
        ) == vec![url("after.png")]
    );
}

/// A declaration that mentions no `url()` contributes nothing, and a `url` that is part of
/// a longer identifier is not a function.
#[test]
fn collects_nothing_from_a_value_without_a_url_function() {
    assert!(from_rule("linear-gradient(red, blue)").is_empty());
    assert!(from_rule("my-url(nope.png)").is_empty());
}

/// An unterminated value is a parse error, not a licence to resynchronise. The quote opens
/// a string token that runs to end-of-input, so a well-formed `url()` spelled *inside* it
/// is string content and was never referenced — collecting it would ship bytes for an
/// address the page does not paint.
#[test]
fn collects_nothing_after_an_unterminated_value() {
    assert!(
        collected(json!([
            r#"#a { background-image: url("open.png, url(after.png)"#
        ]))
        .is_empty()
    );
}

/// A later rule is a separate value, so one malformed rule costs only itself.
#[test]
fn an_unterminated_value_does_not_cost_the_next_rule() {
    assert_eq!(
        collected(json!([
            r#"#a { background-image: url("open.png"#,
            r#"#b { background-image: url("next.png"); }"#
        ])),
        vec![url("next.png")]
    );
}

/// The same reader serves inline computed styles, which is the other half of the collector's
/// reach and the half that carries `getComputedStyle`'s serialisation.
#[test]
fn collects_a_delimiter_bearing_value_from_an_inline_style() {
    let tree = json!({ "tag": "div", "style": { "background-image": r#"url("chart(1).png")"# } });
    let result = node_eval::json(
        &HARNESS
            .replace("__ASSET_ATTRIBUTES__", &super::js_source())
            .replace("__TREE__", &tree.to_string())
            .replace("__CSS_RULES__", "[]")
            .replace("__BASE__", BASE),
    );
    assert_eq!(result["assets"], json!([url("chart(1).png")]));
}
