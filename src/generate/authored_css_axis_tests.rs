use super::authored_css_index::Index;
use crate::model::{Node, Rect, Styles, WritingMode};

/// A box whose sampled style holds the pixels the engine resolved, exactly as a capture
/// records them. The authored percentage exists only in the stylesheet, so the emitted value
/// is right only if it came from there.
fn node(mode: WritingMode, rtl: bool) -> Node {
    let mut node = Node {
        disabled: false,
        rtl,
        writing_mode: mode,
        path: "html>body>div".into(),
        parent: Some("html>body".into()),
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 100.0,
        },
        style: Styles::from([
            ("padding-left".into(), "40px".into()),
            ("padding-right".into(), "40px".into()),
            ("margin-top".into(), "20px".into()),
            ("margin-bottom".into(), "60px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "box".into());
    node
}

/// The defect. The declaration is recognised — the resolver classifies `padding-inline`
/// correctly — and then discarded, because the emission path could name only one property.
/// The sampled pixel stays in its place and the box stops responding to resize.
#[test]
fn an_axis_shorthand_emits_the_authored_value_on_both_edges() {
    let rules = vec![".box{padding-inline:10%;}".into()];
    let styles = Index::new(&rules).declarations(&node(WritingMode::default(), false));

    assert_eq!(styles.get("padding-left").map(String::as_str), Some("10%"));
    assert_eq!(styles.get("padding-right").map(String::as_str), Some("10%"));
    assert!(!styles.contains_key("padding-inline"));
}

/// Two values are not one value repeated. Reading them as one would put the start value on
/// both edges, which is a wrong box rather than a frozen one and would never fail to emit.
#[test]
fn two_values_reach_the_start_and_end_edges_in_that_order() {
    let rules = vec![".box{margin-block:5% 15%;}".into()];
    let styles = Index::new(&rules).declarations(&node(WritingMode::default(), false));

    assert_eq!(styles.get("margin-top").map(String::as_str), Some("5%"));
    assert_eq!(styles.get("margin-bottom").map(String::as_str), Some("15%"));
}

/// The one-edge spelling was already rescued, and this pins the pair: a shorthand and a
/// longhand naming the same edge with the same value must emit the same declaration. If
/// this diverges, the shorthand is being resolved by something other than the shared owner.
#[test]
fn a_shorthand_and_a_longhand_agree_on_the_edge_they_share() {
    let rules = vec![".box{padding-inline:10%;}".into()];
    let shorthand = Index::new(&rules).declarations(&node(WritingMode::default(), false));
    let rules = vec![".box{padding-inline-start:10%;}".into()];
    let longhand = Index::new(&rules).declarations(&node(WritingMode::default(), false));

    assert_eq!(shorthand.get("padding-left"), longhand.get("padding-left"));
}

/// Direction reaches the inline axis alone, so a right-to-left page swaps which physical
/// edge the first value lands on and leaves the block axis where it was.
#[test]
fn direction_swaps_the_inline_edges_and_leaves_the_block_axis_alone() {
    let rules = vec![".box{padding-inline:10% 30%;margin-block:5% 15%;}".into()];
    let styles = Index::new(&rules).declarations(&node(WritingMode::default(), true));

    assert_eq!(styles.get("padding-right").map(String::as_str), Some("10%"));
    assert_eq!(styles.get("padding-left").map(String::as_str), Some("30%"));
    assert_eq!(styles.get("margin-top").map(String::as_str), Some("5%"));
    assert_eq!(styles.get("margin-bottom").map(String::as_str), Some("15%"));
}

/// A page authoring no logical name must be byte-identical, so a physical shorthand keeps
/// reaching the output under its own name rather than being decomposed on the way.
#[test]
fn a_physical_shorthand_is_not_rewritten() {
    let rules = vec![".box{padding:10%;}".into()];
    let styles = Index::new(&rules).declarations(&node(WritingMode::default(), false));

    assert_eq!(styles.get("padding").map(String::as_str), Some("10%"));
    assert!(!styles.contains_key("padding-left"));
}
