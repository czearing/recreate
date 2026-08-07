use super::{
    fills_viewport, is_root,
    measure::{
        compact_control, compact_role_image, fills_parent_content_box, horizontal_padding,
        intrinsic_media,
    },
};
use crate::model::{Node, Styles, Viewport};

pub(super) fn normalize(
    styles: &mut Styles,
    node: &Node,
    parent: Option<&Node>,
    viewport: &Viewport,
    base: Option<(&Node, &Viewport)>,
) {
    if compact_control(node) || compact_role_image(node) || intrinsic_media(node) {
        return;
    }
    if stretches_between_horizontal_edges(node, parent) {
        if base.is_some() {
            styles.insert("width".into(), "auto".into());
        } else {
            styles.remove("width");
        }
        return;
    }
    if stretches_across_grid_track(node, parent) {
        styles.remove("width");
        return;
    }
    if !is_root(node)
        && let Some(parent) = parent.filter(|parent| fills_parent_content_box(node, parent))
    {
        normalize_parent_filling(styles, node, parent, base.is_some());
        return;
    }
    if !fills_viewport(node, viewport) {
        return;
    }
    if !is_root(node) {
        let padding = horizontal_padding(&node.style);
        let width = if node
            .style
            .get("box-sizing")
            .is_some_and(|value| value == "content-box")
            && padding > 0.0
        {
            format!("calc(100% - {padding}px)")
        } else {
            "100%".into()
        };
        styles.insert("width".into(), width);
        return;
    }
    let fixed_base = base.is_some_and(|(node, viewport)| !fills_viewport(node, viewport));
    if fixed_base {
        styles.insert("width".into(), "auto".into());
    } else {
        styles.remove("width");
    }
}

fn normalize_parent_filling(styles: &mut Styles, node: &Node, parent: &Node, has_base: bool) {
    if matches!(
        node.tag.as_str(),
        "button" | "input" | "select" | "textarea"
    ) || parent.style.get("display").map(String::as_str) == Some("flex")
        && parent.style.get("align-items").map(String::as_str) == Some("center")
    {
        styles.insert("width".into(), "100%".into());
    } else if styles
        .get("width")
        .is_some_and(|width| width.ends_with("px"))
    {
        if pinned_along_main_axis(node, parent) {
            // A flex item that cannot grow has a definite inline size and does not track
            // its parent, so the sampled width is the authored width. Keep it.
        } else if needs_explicit_inline_fill(node, parent) {
            styles.insert("width".into(), "100%".into());
        } else if has_base {
            styles.insert("width".into(), "auto".into());
        } else {
            styles.remove("width");
        }
    }
}

/// A flex item laid out along the inline axis only absorbs its parent's free space when it
/// is allowed to grow. Otherwise its width is its own, and coincides with the parent's
/// content box at the sampled viewport only because that is what the author sized it to.
fn pinned_along_main_axis(node: &Node, parent: &Node) -> bool {
    matches!(
        parent.style.get("display").map(String::as_str),
        Some("flex" | "inline-flex")
    ) && !inline_axis_is_cross_axis(parent)
        && !grows_along_main_axis(node)
}

/// Dropping a sampled width is safe only when the box fills its parent on its own. A block
/// child of a block does; a flex item only does so on the cross axis while it is stretched.
/// Under a flex parent that aligns its items to one edge, dropping the width instead
/// collapses the box onto its content, so state the fill explicitly.
fn needs_explicit_inline_fill(node: &Node, parent: &Node) -> bool {
    if !matches!(
        parent.style.get("display").map(String::as_str),
        Some("flex" | "inline-flex")
    ) {
        return false;
    }
    if !inline_axis_is_cross_axis(parent) {
        return false;
    }
    let alignment = node
        .style
        .get("align-self")
        .map(String::as_str)
        .filter(|alignment| *alignment != "auto")
        .or_else(|| parent.style.get("align-items").map(String::as_str))
        .unwrap_or("normal");
    !matches!(alignment, "normal" | "stretch")
}

/// A row flex container lays its items out along the inline axis; a column one lays them
/// out along the block axis, leaving the inline axis as the cross axis.
fn inline_axis_is_cross_axis(parent: &Node) -> bool {
    parent
        .style
        .get("flex-direction")
        .map(String::as_str)
        .unwrap_or("row")
        .starts_with("column")
}

/// A flex item only absorbs its parent's free inline space when it is allowed to grow, or
/// when its basis already asks for the whole line.
fn grows_along_main_axis(node: &Node) -> bool {
    if let Some(grow) = node.style.get("flex-grow") {
        return grow.trim().parse::<f64>().is_ok_and(|grow| grow > 0.0);
    }
    let Some(flex) = node.style.get("flex").map(|flex| flex.trim()) else {
        return false;
    };
    match flex {
        "auto" | "none" | "initial" => flex == "auto",
        _ => flex
            .split_whitespace()
            .next()
            .and_then(|grow| grow.parse::<f64>().ok())
            .is_some_and(|grow| grow > 0.0),
    }
}

fn stretches_across_grid_track(node: &Node, parent: Option<&Node>) -> bool {
    parent.is_some_and(|parent| {
        parent.style.get("display").map(String::as_str) == Some("grid")
            && !matches!(
                node.style.get("position").map(String::as_str),
                Some("absolute" | "fixed")
            )
            && node
                .style
                .get("justify-self")
                .is_none_or(|value| matches!(value.as_str(), "auto" | "normal" | "stretch"))
    })
}

fn stretches_between_horizontal_edges(node: &Node, parent: Option<&Node>) -> bool {
    let Some(parent) = parent else {
        return false;
    };
    if !matches!(
        node.style.get("position").map(String::as_str),
        Some("absolute" | "fixed")
    ) {
        return false;
    }
    let left = node.rect.x - parent.rect.x;
    let right = parent.rect.x + parent.rect.width - node.rect.x - node.rect.width;
    (0.0..=64.0).contains(&left)
        && (0.0..=64.0).contains(&right)
        && node.style.get("left").is_some_and(|value| value != "auto")
        && node.style.get("right").is_some_and(|value| value != "auto")
}
