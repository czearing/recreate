use crate::model::{Node, Viewport};

pub fn role(node: &Node, parent: Option<&Node>, viewport: &Viewport) -> &'static str {
    let right = f64::from(viewport.width) - node.rect.x - node.rect.width;
    if node.rect.width < f64::from(viewport.width) * 0.5
        || node.rect.x <= 1.0
        || (node.rect.x - right).abs() > 16.0
    {
        return "normal";
    }
    let owns_centering = node
        .style
        .get("max-width")
        .is_some_and(|value| value != "none")
        || ["margin-left", "margin-right"].into_iter().any(|key| {
            node.style
                .get(key)
                .and_then(|value| value.strip_suffix("px"))
                .and_then(|value| value.parse::<f64>().ok())
                .is_some_and(|value| value > 1.0)
        });
    if !owns_centering {
        return "normal";
    }
    if parent.is_some_and(|parent| {
        (parent.rect.x - node.rect.x).abs() <= 4.0
            && (parent.rect.width - node.rect.width).abs() <= 4.0
    }) {
        "nested-centered"
    } else {
        "outer-centered"
    }
}
