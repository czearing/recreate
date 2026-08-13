use super::authored_css_index::Index;
use crate::model::{Node, Rect, Styles, WritingMode};

/// A box whose sampled style holds the pixels the engine resolved, exactly as a capture
/// records them. Every authored value here carries `var()`, so each one reaches the gate that
/// decides between the authored text and the sample.
fn node() -> Node {
    let mut node = Node {
        scrollbar_gutter: 0.0,
        disabled: false,
        rtl: false,
        writing_mode: WritingMode::default(),
        blocking_overlay: false,
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
        style: Styles::from([("width".into(), "442px".into())]),
        before: None,
        after: None,
        ..Default::default()
    };
    node.attributes.insert("class".into(), "box".into());
    node
}

fn width(authored: &str) -> Option<String> {
    let rules = vec![format!(".box{{width:{authored};}}")];
    Index::new(&rules).declarations(&node()).remove("width")
}

/// The defect. The authored length tracks the viewport, the sample is the pixels it spanned
/// at the capture width, and the gate kept the sample because its predicate could not see a
/// unit. The two spellings differ only in that unit.
#[test]
fn a_viewport_length_survives_the_gate_exactly_as_a_percentage_does() {
    assert_eq!(
        width("calc(var(--gutter) + 30vw)").as_deref(),
        Some("calc(var(--gutter) + 30vw)")
    );
    assert_eq!(
        width("calc(var(--gutter) + 30%)").as_deref(),
        Some("calc(var(--gutter) + 30%)")
    );
}

/// A container-query length is resolved to pixels and recomputed on container resize just as
/// a viewport length is, and a capture confirmed it freezes into the same per-band staircase.
#[test]
fn a_container_query_length_survives_the_gate_too() {
    assert_eq!(
        width("calc(var(--gutter) + 30cqw)").as_deref(),
        Some("calc(var(--gutter) + 30cqw)")
    );
}

/// The gate exists to prefer the sample, so it must still say no. A fix that keeps every
/// value carrying `var()` has disabled the gate rather than corrected its predicate.
#[test]
fn a_static_reference_is_still_answered_by_the_sample() {
    assert_eq!(width("calc(var(--gutter) + 4px)"), None);
    assert_eq!(width("calc(var(--vwrap) + 40px)"), None);
}

/// A track list responds to width by changing its column count rather than by carrying a
/// unit. It is kept for a separate reason, and that routing must survive the repair.
#[test]
fn an_intrinsically_sized_track_list_still_reaches_the_output() {
    let authored = "repeat(auto-fit, minmax(var(--min-col), 1fr))";
    let rules = vec![format!(".box{{grid-template-columns:{authored};}}")];
    let styles = Index::new(&rules).declarations(&node());

    assert_eq!(
        styles.get("grid-template-columns").map(String::as_str),
        Some(authored)
    );
}
