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
