use super::fills_viewport;
use crate::model::{Node, Styles, Viewport};

pub(super) fn normalize_fixed_edge(styles: &mut Styles, node: &Node, viewport: &Viewport) {
    if node.style.get("position").map(String::as_str) != Some("fixed")
        || node.rect.width >= f64::from(viewport.width) * 0.8
    {
        return;
    }
    let right = f64::from(viewport.width) - node.rect.x - node.rect.width;
    if !(-1.0..=32.0).contains(&right) {
        return;
    }
    styles.insert("left".into(), "auto".into());
    styles.insert("right".into(), format!("{}px", right.max(0.0)));
}

/// An absolutely positioned box given both edge offsets *and* an explicit size is
/// over-constrained: CSS silently drops one edge, so the surviving edge freezes the
/// offset sampled at one viewport. Only act when the two offsets and the size actually
/// reconcile against the containing block, which proves both are measured from it
/// rather than one being derived from the viewport. Restrict this to component-scale
/// containers; at page scale the generator's offsets are not reliably container-relative.
/// Then keep the nearer edge, the real anchor, so the box tracks its container at
/// every width.
pub(super) fn normalize_overconstrained_inset(
    styles: &mut Styles,
    parent: Option<&Node>,
    viewport: &Viewport,
) {
    if !styles
        .get("position")
        .is_some_and(|value| value == "absolute" || value == "fixed")
    {
        return;
    }
    let Some(parent) = parent else { return };
    if !positioned(parent) || parent.rect.width >= f64::from(viewport.width) * 0.5 {
        return;
    }
    for (start, end, size, extent) in [
        ("left", "right", "width", parent.rect.width),
        ("top", "bottom", "height", parent.rect.height),
    ] {
        let (Some(start_offset), Some(end_offset), Some(size_value)) = (
            length(styles, start),
            length(styles, end),
            length(styles, size),
        ) else {
            continue;
        };
        if (start_offset + size_value + end_offset - extent).abs() > 1.5 {
            continue;
        }
        let far = if end_offset < start_offset {
            start
        } else {
            end
        };
        styles.insert(far.into(), "auto".into());
    }
}

fn positioned(node: &Node) -> bool {
    node.style
        .get("position")
        .is_some_and(|value| value != "static")
}

fn length(styles: &Styles, name: &str) -> Option<f64> {
    styles
        .get(name)?
        .strip_suffix("px")?
        .trim()
        .parse::<f64>()
        .ok()
}

pub(super) fn normalize_centering(
    styles: &mut Styles,
    node: &Node,
    parent: Option<&Node>,
    viewport: &Viewport,
) {
    if styles
        .get("margin-left")
        .is_some_and(|value| value == "auto")
        && styles
            .get("margin-right")
            .is_some_and(|value| value == "auto")
    {
        return;
    }
    if fills_viewport(node, viewport) {
        return;
    }
    let right = f64::from(viewport.width) - node.rect.x - node.rect.width;
    if node.rect.width < f64::from(viewport.width) * 0.5
        || node.rect.x <= 1.0
        || (node.rect.x - right).abs() > 16.0
    {
        return;
    }
    if parent.is_some_and(|parent| {
        (parent.rect.x - node.rect.x).abs() <= 4.0
            && (parent.rect.width - node.rect.width).abs() <= 4.0
    }) {
        return;
    }
    if parent.is_some_and(|parent| {
        let left = node.rect.x - parent.rect.x;
        let right = parent.rect.x + parent.rect.width - node.rect.x - node.rect.width;
        left >= 0.0 && right >= 0.0 && (left - right).abs() <= 4.0
    }) {
        return;
    }
    if !owns_centering(node) {
        return;
    }
    if centered_by_parent_alignment(node, parent) {
        return;
    }
    let gutter = right - node.rect.x;
    let centered_width = node.rect.width + gutter;
    styles.insert(
        "margin-left".into(),
        format!("calc((100vw - {centered_width}px) / 2)"),
    );
    styles.insert("margin-right".into(), "auto".into());
    styles.insert("translate".into(), "0px 0px".into());
}

fn owns_centering(node: &Node) -> bool {
    node.style
        .get("max-width")
        .is_some_and(|value| value != "none")
        || ["margin-left", "margin-right"].into_iter().any(|key| {
            node.style
                .get(key)
                .and_then(|value| value.strip_suffix("px"))
                .and_then(|value| value.parse::<f64>().ok())
                .is_some_and(|value| value > 1.0)
        })
}

/// A flex container that already centers this item horizontally needs no
/// viewport-relative margin; adding one shifts the item out of its container
/// at every width except the captured one.
fn centered_by_parent_alignment(node: &Node, parent: Option<&Node>) -> bool {
    let Some(parent) = parent else {
        return false;
    };
    if !parent
        .style
        .get("display")
        .is_some_and(|display| display.ends_with("flex"))
    {
        return false;
    }
    let column = parent
        .style
        .get("flex-direction")
        .is_some_and(|direction| direction.starts_with("column"));
    if column {
        return node
            .style
            .get("align-self")
            .filter(|value| value.as_str() != "auto")
            .or_else(|| parent.style.get("align-items"))
            .is_some_and(|value| value == "center");
    }
    parent
        .style
        .get("justify-content")
        .is_some_and(|value| value == "center")
}
