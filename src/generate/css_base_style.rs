use crate::model::Node;

pub fn append_indexed(
    node: &Node,
    float: Option<&str>,
    width: Option<f64>,
    rules: &super::authored_css::Index<'_>,
    css: &mut String,
) {
    if let Some(value) = float {
        css.push_str(&format!("float:{value};"));
    }
    if let Some(value) = width {
        css.push_str(&format!("width:{value}px;"));
    }
    let line_clamp = node
        .style
        .get("-webkit-line-clamp")
        .is_some_and(|value| value != "none" && value != "0");
    let authored_line_clamp = (!line_clamp && super::css_visual::multiline_text_box(node))
        .then(|| {
            super::authored_css::positive_integer_property_indexed(
                node,
                rules,
                "-webkit-line-clamp",
            )
        })
        .flatten();
    if line_clamp
        && (node
            .style
            .get("-webkit-box-orient")
            .is_some_and(|value| value == "vertical")
            || super::authored_css::has_property_indexed(node, rules, "-webkit-box-orient"))
        || authored_line_clamp.is_some()
    {
        css.push_str("display:-webkit-box;");
        if let Some(lines) = authored_line_clamp {
            css.push_str(&format!(
                "-webkit-box-orient:vertical;-webkit-line-clamp:{lines};"
            ));
        }
    }
}
