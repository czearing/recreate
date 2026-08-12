//! Values derived from measurement rather than from capture.
//!
//! A recreation replays captured declarations; it does not re-derive them. The capture
//! records every computed longhand, so a property the capture already carries can only be
//! duplicated by an inference, or contradicted by one — and the captured value came from
//! the engine that actually laid the page out. The invariant this module must hold is that
//! the properties inferred here and the properties the capture records stay disjoint.
//!
//! A `flex-direction` reversal inferred from geometry used to live here and violated it. It
//! compared the first and last child by position, over a child set that filtered nothing, so
//! an absolutely positioned child — which flex layout does not lay out at all, per CSS
//! Flexbox 4.1 — decided the main axis. Its guess was appended after the captured value in
//! the same block, where source order let it win. Every input to the painted order of flex
//! items is recovered by the recreation: `flex-direction`, `order` and `flex-wrap` are
//! captured as declarations, while `direction` and `writing-mode` are inherited and so are
//! captured as declarations on the element that declared them and as resolved per-node
//! facts everywhere else. The correction it could have legitimately carried was always
//! empty.

use crate::model::Node;

pub fn multiline_text_box(node: &Node) -> bool {
    node.style
        .get("line-height")
        .and_then(|value| value.strip_suffix("px"))
        .and_then(|value| value.parse::<f64>().ok())
        .is_some_and(|line_height| node.rect.height > line_height * 1.5)
}

pub fn important_interaction_paint(css: &str) -> String {
    css.split_inclusive(';')
        .map(|declaration| {
            let property = declaration
                .split_once(':')
                .map(|(property, _)| property)
                .unwrap_or_default();
            if (matches!(
                property,
                "background-color"
                    | "border"
                    | "color"
                    | "fill"
                    | "stroke"
                    | "-webkit-text-fill-color"
            ) || property.starts_with("border-"))
                && !declaration.contains("!important")
            {
                format!("{}!important;", declaration.trim_end_matches(';'))
            } else {
                declaration.to_string()
            }
        })
        .collect()
}

/// `float` is a captured property, so this shares the shape the module comment warns about
/// and is retained only because a zero-width static block at its parent's right edge is a
/// float the capture cannot express: `getComputedStyle` reports the used value `none` for a
/// floated box the layout has already collapsed.
pub fn inferred_float(node: &Node, parent: Option<&Node>) -> Option<&'static str> {
    let parent = parent?;
    let missing_float = node.style.get("float").is_none_or(|value| value == "none");
    let right_edge = parent.rect.x + parent.rect.width;
    (missing_float
        && parent
            .style
            .get("display")
            .is_some_and(|value| value == "block")
        && node
            .style
            .get("display")
            .is_some_and(|value| value == "block")
        && node
            .style
            .get("position")
            .is_some_and(|value| value == "static")
        && node.rect.width <= 0.5
        && (node.rect.x - right_edge).abs() <= 1.0)
        .then_some("right")
}
