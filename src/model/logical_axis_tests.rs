use super::{Physical, WritingMode, physical_property};

/// The case the empty-string sentinel used to hide, and which the `Unsupported` variant then
/// reported rather than honoured. A logical shorthand carries one or two values across both
/// edges of an axis, so it resolves to a pair of declarations rather than to a single name.
/// Every name here keeps whatever suffix its family carries on both edges.
#[test]
fn a_logical_shorthand_over_both_edges_resolves_to_both_of_them() {
    for (name, start, end) in [
        ("margin-inline", "margin-left", "margin-right"),
        ("margin-block", "margin-top", "margin-bottom"),
        ("padding-inline", "padding-left", "padding-right"),
        ("inset-block", "top", "bottom"),
        ("border-inline", "border-left", "border-right"),
        (
            "border-block-width",
            "border-top-width",
            "border-bottom-width",
        ),
        (
            "border-inline-color",
            "border-left-color",
            "border-right-color",
        ),
    ] {
        assert_eq!(
            physical_property(WritingMode::default(), false, name),
            Physical::Axis(start.to_string(), end.to_string()),
            "{name}"
        );
    }
}

/// One value covers both edges; two name them in start-then-end order. The order is a fact
/// about the shorthand and never varies — only which physical edge is the start does.
#[test]
fn one_value_covers_both_edges_and_two_are_read_start_then_end() {
    assert_eq!(
        physical_property(WritingMode::default(), false, "padding-inline")
            .into_declarations("padding-inline", "10%"),
        vec![
            ("padding-left".to_string(), "10%".to_string()),
            ("padding-right".to_string(), "10%".to_string()),
        ]
    );
    assert_eq!(
        physical_property(WritingMode::default(), false, "margin-block")
            .into_declarations("margin-block", "5% 15%"),
        vec![
            ("margin-top".to_string(), "5%".to_string()),
            ("margin-bottom".to_string(), "15%".to_string()),
        ]
    );
}

/// `sideways-lr` runs its inline axis bottom-to-top, so its start edge is the one every
/// other vertical mode calls the end. A shorthand that assumed the vertical modes agree
/// would put the author's start value on the wrong edge without ever failing to emit.
#[test]
fn the_start_edge_of_an_axis_follows_the_mode_rather_than_the_order_written() {
    assert_eq!(
        physical_property(WritingMode::SidewaysLr, false, "margin-inline")
            .into_declarations("margin-inline", "5% 15%"),
        vec![
            ("margin-bottom".to_string(), "5%".to_string()),
            ("margin-top".to_string(), "15%".to_string()),
        ]
    );
    assert_eq!(
        physical_property(WritingMode::VerticalLr, false, "margin-inline")
            .into_declarations("margin-inline", "5% 15%"),
        vec![
            ("margin-top".to_string(), "5%".to_string()),
            ("margin-bottom".to_string(), "15%".to_string()),
        ]
    );
}

/// Not every axis shorthand divides its value between the edges. `border-inline` draws the
/// whole border on each one, so a component count the edge reading does not recognise is
/// carried across intact rather than torn into pieces that mean something else.
#[test]
fn a_value_that_is_not_a_pair_of_edges_reaches_both_edges_whole() {
    assert_eq!(
        physical_property(WritingMode::default(), false, "border-inline")
            .into_declarations("border-inline", "1px solid red"),
        vec![
            ("border-left".to_string(), "1px solid red".to_string()),
            ("border-right".to_string(), "1px solid red".to_string()),
        ]
    );
    assert_eq!(
        physical_property(WritingMode::default(), false, "padding-inline")
            .into_declarations("padding-inline", "calc(1rem + 2px)"),
        vec![
            ("padding-left".to_string(), "calc(1rem + 2px)".to_string()),
            ("padding-right".to_string(), "calc(1rem + 2px)".to_string()),
        ]
    );
}

/// A query names the declaration it is looking for, and a shorthand is still the
/// declaration that is there. Resolving it to its edges is an emission concern, so
/// answering must not follow it there — the inset arbiter asks for `inset-inline` by name.
#[test]
fn an_axis_shorthand_answers_a_query_for_itself_and_not_for_its_edges() {
    let physical = physical_property(WritingMode::default(), false, "inset-inline");
    assert!(physical.answers("inset-inline", "inset-inline"));
    assert!(!physical.answers("inset-inline", "left"));
    assert!(!physical.answers("inset-inline", "right"));
}
