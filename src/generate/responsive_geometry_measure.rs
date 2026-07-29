use crate::model::{Node, Styles};

pub(super) fn fills_parent_content_box(node: &Node, parent: &Node) -> bool {
    let left = border_px(&parent.style, "border-left-width", "border-left")
        + px(&parent.style, "padding-left").unwrap_or_default();
    let right = border_px(&parent.style, "border-right-width", "border-right")
        + px(&parent.style, "padding-right").unwrap_or_default();
    let content_width = if parent
        .style
        .get("box-sizing")
        .is_some_and(|value| value == "border-box")
    {
        parent.rect.width - left - right
    } else {
        px(&parent.style, "width").unwrap_or(parent.rect.width - left - right)
    };
    (node.rect.x - parent.rect.x - left).abs() <= 1.0
        && (node.rect.width - content_width).abs() <= 1.0
}

pub(super) fn horizontal_padding(styles: &Styles) -> f64 {
    let values: Vec<_> = styles
        .get("padding")
        .into_iter()
        .flat_map(|value| value.split_whitespace())
        .filter_map(|value| value.strip_suffix("px")?.parse::<f64>().ok())
        .collect();
    match values.as_slice() {
        [all] => all * 2.0,
        [_, horizontal] | [_, horizontal, _] => horizontal * 2.0,
        [_, right, _, left] => right + left,
        _ => 0.0,
    }
}

pub(super) fn intrinsic_media(node: &Node) -> bool {
    matches!(
        node.tag.as_str(),
        "canvas"
            | "circle"
            | "ellipse"
            | "foreignObject"
            | "image"
            | "img"
            | "line"
            | "path"
            | "polygon"
            | "polyline"
            | "rect"
            | "svg"
            | "text"
            | "use"
            | "video"
    )
}

pub(super) fn compact_control(node: &Node) -> bool {
    node.rect.width <= 48.0
        && node.rect.height <= 48.0
        && (matches!(node.tag.as_str(), "button" | "input" | "select")
            || node
                .attributes
                .get("role")
                .is_some_and(|role| role == "button"))
}

pub(super) fn compact_role_image(node: &Node) -> bool {
    node.rect.width <= 80.0
        && node.rect.height <= 80.0
        && node
            .attributes
            .get("role")
            .is_some_and(|role| role == "img")
}

pub(super) fn px(styles: &Styles, key: &str) -> Option<f64> {
    styles.get(key)?.strip_suffix("px")?.parse::<f64>().ok()
}

fn border_px(styles: &Styles, width: &str, side: &str) -> f64 {
    styles
        .get(width)
        .or_else(|| styles.get(side))
        .or_else(|| styles.get("border"))
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.strip_suffix("px"))
        .and_then(|value| value.parse().ok())
        .unwrap_or_default()
}
