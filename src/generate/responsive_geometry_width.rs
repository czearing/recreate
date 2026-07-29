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
        if has_base {
            styles.insert("width".into(), "auto".into());
        } else {
            styles.remove("width");
        }
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
