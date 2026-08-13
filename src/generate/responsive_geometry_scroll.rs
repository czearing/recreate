use super::measure::px;
use crate::model::{Node, Styles, Viewport};

/// Below this a difference is a rounding artefact of subpixel layout, not a scrollbar.
const MIN_GUTTER: f64 = 6.0;

/// How much wider the pane must be emitted than the width the engine resolved for it.
///
/// A scrollbar is laid out between the padding edge and the content edge, and the engine
/// reports it by shrinking the resolved value: a 300px content-box pane with a 10px
/// scrollbar resolves `width` to 290px. Emitting that hands the recreation a pane one
/// scrollbar narrower than the source, and the recreation then takes its own scrollbar out
/// of that, so the loss compounds. Adding the gutter back restores the authored width.
///
/// The gutter is measured at capture, never inferred from `overflow-y`, because the keyword
/// and the gutter answer different questions. `scroll` reserves a gutter unconditionally
/// where `auto` reserves one only while the content overflows; `scrollbar-gutter: stable`
/// reserves one with no scrollbar shown at all; and a specified `visible` computes to `auto`
/// whenever the other axis is scrollable, so the declaration that made the box scroll need
/// not be on this axis. Meanwhile `hidden` and `clip` count as scrolling under other rules
/// and reserve nothing here. No reading of one keyword separates those five, and widening
/// the allow-list only moves which of them is wrong. A measurement separates all five,
/// because a box that reserved nothing measures zero. It is also indifferent to
/// `scrollbar-width`: the loss is the whole scrollbar, not the difference between a thin one
/// and a default one.
///
/// Only a content-box pane lost anything. Under `border-box` the resolved width already
/// spans the border box and the scrollbar sits inside it, so adding the gutter would invent
/// space. `box-sizing` is absent from a pruned style exactly when it is the initial
/// `content-box`, so reading it as content-box unless it says otherwise is sound.
fn gutter(node: &Node) -> f64 {
    if node.style.get("box-sizing").map(String::as_str) == Some("border-box") {
        return 0.0;
    }
    node.scrollbar_gutter
}

fn has_gutter(node: &Node) -> bool {
    gutter(node) >= MIN_GUTTER
}

pub(super) fn preserve_space(
    styles: &mut Styles,
    node: &Node,
    base: Option<&Node>,
    viewport: &Viewport,
) {
    // A pane spanning most of the viewport is sized by the width stage against the viewport
    // rather than by this pixel width, so adding the gutter here would fight it.
    let sized_here = node.rect.width < f64::from(viewport.width) * 0.8;
    if sized_here && has_gutter(node) {
        if let Some(width) = px(&node.style, "width") {
            styles.insert("width".into(), format!("{}px", width + gutter(node)));
        }
        return;
    }
    // The gutter is gone at this viewport but the base reserved one, so the base's border
    // was measured against a box this one no longer has.
    if base.is_some_and(has_gutter) {
        remove_border(styles);
    }
}

fn remove_border(styles: &mut Styles) {
    styles.insert("border-right-width".into(), "0px".into());
    styles.insert("border-right-style".into(), "none".into());
}

#[cfg(test)]
#[path = "responsive_geometry_scroll_tests.rs"]
mod tests;
