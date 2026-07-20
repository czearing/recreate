use crate::model::{Node, Styles, Viewport};

pub fn normalize(
    styles: &mut Styles,
    node: &Node,
    parent: Option<&Node>,
    viewport: &Viewport,
    base: Option<(&Node, &Viewport)>,
) {
    preserve_scrollbar_space(styles, node, base.map(|(node, _)| node), viewport);
    normalize_width(styles, node, parent, viewport, base);
    super::responsive_height::normalize(styles, node, viewport);
    let authored_centering = styles
        .get("margin-left")
        .is_some_and(|value| value == "auto")
        && styles
            .get("margin-right")
            .is_some_and(|value| value == "auto");
    if !is_root(node)
        && !authored_centering
        && base.is_some_and(|(node, viewport)| centered(node, viewport))
    {
        styles.insert("margin-left".into(), "0px".into());
        styles.insert("margin-right".into(), "0px".into());
    }

    normalize_centering(styles, node, parent, viewport);
}

fn preserve_scrollbar_space(
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
        if base.is_some_and(has_scrollbar_gutter) {
            styles.insert("border-right-width".into(), "0px".into());
            styles.insert("border-right-style".into(), "none".into());
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
        styles.insert("border-right-width".into(), format!("{gutter}px"));
        styles.insert("border-right-style".into(), "solid".into());
        styles.insert("border-right-color".into(), "transparent".into());
    } else if base.is_some_and(has_scrollbar_gutter) {
        styles.insert("border-right-width".into(), "0px".into());
        styles.insert("border-right-style".into(), "none".into());
    }
}

fn has_scrollbar_gutter(node: &Node) -> bool {
    let Some(width) = px(&node.style, "width") else {
        return false;
    };
    node.rect.width
        - width
        - px(&node.style, "padding-left").unwrap_or_default()
        - px(&node.style, "padding-right").unwrap_or_default()
        >= 6.0
}

fn px(styles: &Styles, key: &str) -> Option<f64> {
    styles.get(key)?.strip_suffix("px")?.parse::<f64>().ok()
}

fn normalize_width(
    styles: &mut Styles,
    node: &Node,
    parent: Option<&Node>,
    viewport: &Viewport,
    base: Option<(&Node, &Viewport)>,
) {
    if compact_control(node) || intrinsic_media(node) {
        return;
    }
    if !is_root(node) && parent.is_some_and(|parent| fills_parent_content_box(node, parent)) {
        if matches!(
            node.tag.as_str(),
            "button" | "input" | "select" | "textarea"
        ) {
            styles.insert("width".into(), "100%".into());
            return;
        }
        if parent.is_some_and(|parent| {
            parent.style.get("display").map(String::as_str) == Some("flex")
                && parent.style.get("align-items").map(String::as_str) == Some("center")
        }) {
            styles.insert("width".into(), "100%".into());
            return;
        }
        if styles
            .get("width")
            .is_some_and(|width| width.ends_with("px"))
        {
            if base.is_some() {
                styles.insert("width".into(), "auto".into());
            } else {
                styles.remove("width");
            }
        }
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

fn fills_parent_content_box(node: &Node, parent: &Node) -> bool {
    let left = px(&parent.style, "border-left-width").unwrap_or_default()
        + px(&parent.style, "padding-left").unwrap_or_default();
    let right = px(&parent.style, "border-right-width").unwrap_or_default()
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

fn intrinsic_media(node: &Node) -> bool {
    matches!(node.tag.as_str(), "canvas" | "img" | "svg" | "video")
}

fn compact_control(node: &Node) -> bool {
    node.rect.width <= 48.0
        && node.rect.height <= 48.0
        && (matches!(node.tag.as_str(), "button" | "input" | "select")
            || node
                .attributes
                .get("role")
                .is_some_and(|role| role == "button"))
}

fn horizontal_padding(styles: &Styles) -> f64 {
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

fn normalize_centering(
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

fn centered(node: &Node, viewport: &Viewport) -> bool {
    let right = f64::from(viewport.width) - node.rect.x - node.rect.width;
    node.rect.width >= f64::from(viewport.width) * 0.5
        && node.rect.x > 1.0
        && (node.rect.x - right).abs() <= 16.0
}

fn fills_viewport(node: &Node, viewport: &Viewport) -> bool {
    let viewport_width = f64::from(viewport.width);
    let right_inset = viewport_width - node.rect.x - node.rect.width;
    (node.rect.width - viewport_width).abs() <= 1.0 && node.rect.x.abs() <= 1.0
        || is_root(node)
            && (node.rect.x - right_inset).abs() <= 1.0
            && node.rect.x.abs() <= 32.0
            && right_inset.abs() <= 32.0
}

fn is_root(node: &Node) -> bool {
    matches!(node.tag.as_str(), "html" | "body")
        || node.attributes.get("id").is_some_and(|id| id == "root")
}
