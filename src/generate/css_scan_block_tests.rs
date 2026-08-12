use super::{block, grammatical};

/// The defect this reader was extended to end. CSS lets a declaration value hold a brace
/// inside a quoted string, so a reader that splits on the byte cuts the rule in half and
/// leaves the string open. Everything after the opening quote is data until it closes.
#[test]
fn a_brace_inside_a_quoted_value_does_not_end_a_block() {
    let rule = r#".alpha { font-family: "A}B", sans-serif; color: red; }"#;
    let (before, inside, after) = block(rule).expect("a block");
    assert_eq!(before, ".alpha ");
    assert_eq!(inside, r#" font-family: "A}B", sans-serif; color: red; "#);
    assert_eq!(after, "");
}

/// A block ends at the closer matching its own opener, not at the first closer in the
/// bytes. Trailing-brace trimming cannot express this: it strips whatever braces end the
/// text, so a nested block's closer and the outer block's closer are indistinguishable.
#[test]
fn a_nested_block_does_not_end_the_block_containing_it() {
    let rule = "@media (min-width: 10px) { .alpha { color: red; } }";
    let (before, inside, _) = block(rule).expect("a block");
    assert_eq!(before, "@media (min-width: 10px) ");
    assert_eq!(inside, " .alpha { color: red; } ");
}

/// A media body holds several rules, so the remainder must be returned rather than
/// discarded. A reader that stopped at the first block would drop every later rule.
#[test]
fn the_remainder_after_a_block_carries_the_blocks_that_follow() {
    let body = " .alpha { color: red; } .bravo { color: blue; } ";
    let (_, first, rest) = block(body).expect("first block");
    assert_eq!(first, " color: red; ");
    let (selector, second, last) = block(rest).expect("second block");
    assert_eq!(selector.trim(), ".bravo");
    assert_eq!(second, " color: blue; ");
    assert_eq!(block(last), None);
}

/// An unterminated string never yields a block, because its closer is data. Reporting one
/// anyway would emit the truncation this reader exists to prevent.
#[test]
fn an_unterminated_string_swallows_the_closer_rather_than_ending_a_block() {
    assert_eq!(block(r#".alpha { content: "} "#), None);
    assert_eq!(block(".alpha color: red;"), None);
}

/// Braces now count towards depth, which selector text never contains outside a quoted
/// value. The answers a selector caller gets must therefore be unchanged.
#[test]
fn counting_braces_does_not_move_any_answer_a_selector_asks() {
    let selector = "div.card:is(.a, .b)[data-x=\"{\"]";
    assert_eq!(grammatical(selector, '.'), Some(3));
    assert_eq!(grammatical(selector, '{'), None);
    assert_eq!(grammatical(selector, ']'), Some(selector.len() - 1));
}
