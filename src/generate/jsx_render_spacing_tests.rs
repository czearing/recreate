//! Compensation must be the residual left after what the emitter already reproduces.
//!
//! The spacer path exists to rescue geometry the emitter genuinely cannot express, so these
//! tests assert both directions: the fabricated cases are refused, and the rescued case still
//! fires. Geometry is transcribed from the `fabricated-spacer` capture and from the one place
//! in the scene corpus where the path legitimately fires.

use super::jsx_render_spacing::{leading_placeholder_extent, placeholder_extent};
use super::tree::Components;
use crate::model::{Node, Rect};
use std::collections::BTreeMap;

fn node(path: &str, tag: &str, text: &str, (x, y): (f64, f64), style: &[(&str, &str)]) -> Node {
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: path.into(),
        parent: None,
        tag: tag.into(),
        text: text.into(),
        attributes: Default::default(),
        rect: Rect {
            x,
            y,
            width: 320.0,
            height: 18.0,
        },
        style: style
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
        before: None,
        after: None,
        ..Default::default()
    }
}

/// A parent at the origin whose children are given as `(path suffix, tag, text, position)`.
/// The block child always declares `display:flex`, since a `display` equal to the tag's own
/// default is pruned from the captured style map and never reaches this function.
fn container(
    parent_style: &[(&str, &str)],
    children: &[(&str, &str, &str, (f64, f64))],
) -> (Components, Vec<String>) {
    let mut components = Components {
        items: Vec::new(),
        by_root: BTreeMap::new(),
        children: BTreeMap::new(),
        classes: BTreeMap::new(),
        nodes: BTreeMap::new(),
    };
    components
        .nodes
        .insert("p".into(), node("p", "div", "", (0.0, 0.0), parent_style));
    let mut paths = Vec::new();
    for (suffix, tag, text, position) in children {
        let path = format!("p>{suffix}");
        let style: &[(&str, &str)] = if *tag == "#text" {
            &[]
        } else {
            &[("display", "flex")]
        };
        components
            .nodes
            .insert(path.clone(), node(&path, tag, text, *position, style));
        paths.push(path);
    }
    components.children.insert("p".into(), paths.clone());
    (components, paths)
}

/// Container A of the `fabricated-spacer` scene. The leading text run produces an anonymous
/// block box whose line box is the whole 18px offset, and `render_children` emits that text.
/// Compensating as well would double-count it.
#[test]
fn leading_text_run_explains_the_offset() {
    let (components, children) = container(
        &[],
        &[
            ("#text(1)", "#text", "Label", (0.0, 0.0)),
            ("div:nth-of-type(1)", "div", "", (0.0, 18.0)),
        ],
    );
    assert_eq!(
        leading_placeholder_extent("p", &children, &components),
        None
    );
}

/// A whitespace-only run generates no line box, so it explains nothing and the offset is
/// genuinely unaccounted for. Guarding on "the sibling is text" rather than on the content it
/// renders would silently lose this rescue.
#[test]
fn whitespace_only_leading_text_explains_nothing() {
    let (components, children) = container(
        &[],
        &[
            ("#text(1)", "#text", "\n  ", (0.0, 0.0)),
            ("div:nth-of-type(1)", "div", "", (0.0, 420.0)),
        ],
    );
    assert_eq!(
        leading_placeholder_extent("p", &children, &components),
        Some(420.0)
    );
}

/// Container C. Rects come from `getBoundingClientRect`, which is the border box, so the
/// parent's content edge is `rect + border + padding`. The border is re-emitted verbatim in
/// the stylesheet, so charging it to a spacer reproduces it twice.
#[test]
fn parent_border_explains_the_offset() {
    let (components, children) = container(
        &[("border-top-width", "24px")],
        &[("div:nth-of-type(1)", "div", "", (0.0, 24.0))],
    );
    assert_eq!(
        leading_placeholder_extent("p", &children, &components),
        None
    );
}

/// The same omission on the other axis refuses a legitimate rescue: an unsubtracted
/// `border-left` pushes the child off the content edge and fails the alignment test.
#[test]
fn parent_border_does_not_break_alignment() {
    let (components, children) = container(
        &[("border-left-width", "24px")],
        &[("div:nth-of-type(1)", "div", "", (24.0, 420.0))],
    );
    assert_eq!(
        leading_placeholder_extent("p", &children, &components),
        Some(420.0)
    );
}

/// The one firing in the whole scene corpus (`nso-1`): a scrolled container whose only child
/// sits 420px down with nothing preceding it. Nothing the emitter renders explains this, so
/// the path must still compensate. A blunt deletion would regress exactly this.
#[test]
fn unexplained_offset_is_still_compensated() {
    let (components, children) = container(&[], &[("div:nth-of-type(1)", "div", "", (0.0, 420.0))]);
    assert_eq!(
        leading_placeholder_extent("p", &children, &components),
        Some(420.0)
    );
}

/// Container B, the negative control: the child sits on the content edge, so there is no
/// offset to explain and no spacer to emit.
#[test]
fn child_on_the_content_edge_needs_no_spacer() {
    let (components, children) = container(&[], &[("div:nth-of-type(1)", "div", "", (0.0, 0.0))]);
    assert_eq!(
        leading_placeholder_extent("p", &children, &components),
        None
    );
}

/// The gap filler shares the content-edge expression, so it inherits the same defect: the
/// parent's border is not a gap left by missing siblings.
#[test]
fn gap_extent_excludes_the_parent_border() {
    let (components, children) = container(
        &[("border-top-width", "24px")],
        &[("div:nth-of-type(2)", "div", "", (0.0, 24.0))],
    );
    assert_eq!(placeholder_extent("p", &children[0], 1, &components), None);
}
