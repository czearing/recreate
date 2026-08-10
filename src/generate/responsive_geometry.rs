#[path = "responsive_geometry_measure.rs"]
mod measure;
#[path = "responsive_geometry_position.rs"]
mod position;
#[path = "responsive_geometry_scroll.rs"]
mod scroll;
#[path = "responsive_geometry_width.rs"]
mod width;

use crate::model::{Node, Styles, Viewport};

pub fn normalize(
    styles: &mut Styles,
    node: &Node,
    parent: Option<&Node>,
    viewport: &Viewport,
    base: Option<(&Node, &Viewport)>,
) {
    scroll::preserve_space(styles, node, base.map(|(node, _)| node), viewport);
    width::normalize(styles, node, parent, viewport, base);
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
    position::normalize_centering(styles, node, parent, viewport);
    position::normalize_fixed_edge(styles, node, viewport);
    position::normalize_overconstrained_inset(styles, parent, viewport);
}

pub(super) fn fills_viewport(node: &Node, viewport: &Viewport) -> bool {
    let viewport_width = f64::from(viewport.width);
    let right_inset = viewport_width - node.rect.x - node.rect.width;
    (node.rect.width - viewport_width).abs() <= 1.0 && node.rect.x.abs() <= 1.0
        || is_root(node)
            && (node.rect.x - right_inset).abs() <= 1.0
            && node.rect.x.abs() <= 32.0
            && right_inset.abs() <= 32.0
}

pub(super) fn is_root(node: &Node) -> bool {
    matches!(node.tag.as_str(), "html" | "body")
        || node.attributes.get("id").is_some_and(|id| id == "root")
}

fn centered(node: &Node, viewport: &Viewport) -> bool {
    let right = f64::from(viewport.width) - node.rect.x - node.rect.width;
    node.rect.width >= f64::from(viewport.width) * 0.5
        && node.rect.x > 1.0
        && (node.rect.x - right).abs() <= 16.0
}
