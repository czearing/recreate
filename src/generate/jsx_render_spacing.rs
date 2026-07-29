use super::{
    jsx_attrs::{jsx_tag, void_tag},
    tree::Components,
};

pub(super) fn needs_text_space(
    previous: &crate::model::Node,
    current: &crate::model::Node,
) -> bool {
    if current.tag != "#text"
        || previous
            .text
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
        || current.text.chars().next().is_some_and(char::is_whitespace)
    {
        return false;
    }
    let same_line = (previous.rect.y - current.rect.y).abs() <= 1.0;
    let gap = current.rect.x - (previous.rect.x + previous.rect.width);
    same_line && gap > 0.5 && gap <= 12.0
}

pub(super) fn placeholder_extent(
    parent: &str,
    child: &str,
    missing: usize,
    components: &Components,
) -> Option<(&'static str, f64)> {
    if missing == 0 {
        return None;
    }
    let parent = components.nodes.get(parent)?;
    let child = components.nodes.get(child)?;
    let horizontal = parent
        .style
        .get("display")
        .is_some_and(|value| value == "flex")
        && parent
            .style
            .get("flex-direction")
            .is_some_and(|value| value.starts_with("row"));
    let (axis, offset) = if horizontal {
        (
            "width",
            child.rect.x
                - parent.rect.x
                - pixel_style(parent, "padding-left")
                - pixel_style(child, "margin-left"),
        )
    } else {
        (
            "height",
            child.rect.y
                - parent.rect.y
                - pixel_style(parent, "padding-top")
                - pixel_style(child, "margin-top"),
        )
    };
    let extent = offset / missing as f64;
    (extent > 0.0).then_some((axis, extent))
}

pub(super) fn leading_placeholder_extent(
    parent: &str,
    children: &[String],
    components: &Components,
) -> Option<f64> {
    let parent = components.nodes.get(parent)?;
    if parent
        .style
        .get("display")
        .is_some_and(|value| value != "block")
    {
        return None;
    }
    let child = children
        .iter()
        .filter_map(|path| components.nodes.get(path))
        .find(|node| node.tag != "#text")?;
    if child
        .style
        .get("display")
        .is_none_or(|value| value != "block" && value != "flex" && value != "grid")
        || sibling_index(children.iter().find(|path| {
            components
                .nodes
                .get(*path)
                .is_some_and(|node| node.tag != "#text")
        })?)?
        .1 != 1
        || pixel_style(child, "margin-top") != 0.0
    {
        return None;
    }
    let expected_x = parent.rect.x + pixel_style(parent, "padding-left");
    if (child.rect.x - expected_x).abs() > 0.5 {
        return None;
    }
    let offset = child.rect.y - parent.rect.y - pixel_style(parent, "padding-top");
    (offset >= 12.0).then_some(offset)
}

pub(super) fn pixel_style(node: &crate::model::Node, property: &str) -> f64 {
    node.style
        .get(property)
        .and_then(|value| value.strip_suffix("px"))
        .and_then(|value| value.parse().ok())
        .unwrap_or(0.0)
}

pub(super) fn sibling_index(path: &str) -> Option<(&str, usize)> {
    let segment = path.rsplit('>').next()?;
    let (tag, index) = segment.split_once(":nth-of-type(")?;
    Some((tag, index.strip_suffix(')')?.parse().ok()?))
}

pub(super) fn placeholder(tag: &str, depth: usize, extent: Option<(&str, f64)>) -> String {
    let indent = "  ".repeat(depth);
    let tag = jsx_tag(tag);
    let style = extent.map_or_else(String::new, |(axis, extent)| {
        let mut style = String::from(" style={{");
        style.push_str(&format!(
            "{axis}:\"{extent:.4}px\",flex:\"0 0 {extent:.4}px\""
        ));
        style.push_str("}}");
        style
    });
    if void_tag(tag) {
        format!("{indent}<{tag} data-recreate-startup=\"true\"{style} />\n")
    } else {
        format!("{indent}<{tag} data-recreate-startup=\"true\"{style}></{tag}>\n")
    }
}

pub(super) fn leading_placeholder(depth: usize, extent: f64) -> String {
    let indent = "  ".repeat(depth);
    format!(
        "{indent}<span data-recreate-startup=\"true\" style={{{{display:\"block\",height:\"{extent:.4}px\"}}}}></span>\n"
    )
}
