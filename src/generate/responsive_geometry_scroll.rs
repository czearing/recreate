use super::measure::px;
use crate::model::{Node, Styles, Viewport};

pub(super) fn preserve_space(
    styles: &mut Styles,
    node: &Node,
    base: Option<&Node>,
    viewport: &Viewport,
) {
    if node.style.get("scrollbar-width").map(String::as_str) != Some("thin")
        || node.style.get("overflow-y").map(String::as_str) != Some("auto")
    {
        return;
    }
    if node.rect.width >= f64::from(viewport.width) * 0.8 {
        if base.is_some_and(has_gutter) {
            remove_border(styles);
        }
        return;
    }
    let Some(width) = px(&node.style, "width") else {
        return;
    };
    let expected = width
        + px(&node.style, "padding-left").unwrap_or_default()
        + px(&node.style, "padding-right").unwrap_or_default();
    let gutter = node.rect.width - expected;
    if gutter >= 6.0 {
        styles.insert("width".into(), format!("{}px", width + gutter));
    } else if base.is_some_and(has_gutter) {
        remove_border(styles);
    }
}

fn has_gutter(node: &Node) -> bool {
    let Some(width) = px(&node.style, "width") else {
        return false;
    };
    node.rect.width
        - width
        - px(&node.style, "padding-left").unwrap_or_default()
        - px(&node.style, "padding-right").unwrap_or_default()
        >= 6.0
}

fn remove_border(styles: &mut Styles) {
    styles.insert("border-right-width".into(), "0px".into());
    styles.insert("border-right-style".into(), "none".into());
}
