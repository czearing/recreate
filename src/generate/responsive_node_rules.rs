use super::super::style_delta::changed_styles;
use super::flex::{fluid_flex_item, shrunk_flex_item};
use crate::{
    generate::css::declarations,
    model::{Node, Pseudo, Styles, Viewport},
};
use std::collections::BTreeMap;

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(in crate::generate) fn append_node_rules(
    base: &Node,
    node: &Node,
    parent: Option<&Node>,
    viewports: (&Viewport, &Viewport),
    class: &str,
    assets: &BTreeMap<String, String>,
    css_rules: &[String],
    fluid_height: bool,
    constrained_by_flex: bool,
) -> String {
    append_node_rules_indexed(
        base,
        node,
        parent,
        viewports,
        class,
        assets,
        &super::super::authored_css::Index::new(css_rules),
        fluid_height,
        constrained_by_flex,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn append_node_rules_indexed(
    base: &Node,
    node: &Node,
    parent: Option<&Node>,
    viewports: (&Viewport, &Viewport),
    class: &str,
    assets: &BTreeMap<String, String>,
    css_rules: &super::super::authored_css::Index<'_>,
    fluid_height: bool,
    constrained_by_flex: bool,
) -> String {
    let mut rules = String::new();
    let (base_viewport, viewport) = viewports;
    let mut changed = changed_styles(&base.style, &node.style);
    super::super::authored_css::normalize_indexed(&mut changed, node, css_rules);
    normalize_line_clamp(&mut changed, node, css_rules);
    if constrained_by_flex
        && node
            .style
            .get("flex-shrink")
            .and_then(|value| value.parse::<f64>().ok())
            .is_some_and(|value| value > 0.0)
        && node.rect.width + 1.0 < base.rect.width
    {
        changed.remove("width");
        changed.remove("inline-size");
    }
    if !super::super::authored_css::has_property_indexed(node, css_rules, "width")
        && fluid_flex_item(node, parent)
    {
        changed.remove("width");
        if shrunk_flex_item(base, node, parent)
            && !super::super::authored_css::has_property_indexed(node, css_rules, "min-width")
        {
            changed.insert("min-width".into(), "0".into());
        }
    }
    if fluid_height {
        changed.remove("height");
    }
    super::super::inherited_styles::normalize_indexed(&mut changed, node, parent, css_rules);
    super::super::responsive_geometry::normalize(
        &mut changed,
        node,
        parent,
        viewport,
        Some((base, base_viewport)),
    );
    super::super::style_delta::append_reversions(
        &mut changed,
        &super::super::style_delta::declared(&base.style),
        &node.style,
    );
    if !changed.is_empty() {
        rules.push_str(&format!(".{class}{{{}}}", declarations(&changed, assets)));
    }
    for (suffix, base, current) in super::super::css_pseudo::paired(&base.pseudos, &node.pseudos) {
        append_pseudo_rule(class, suffix, base, current, assets, &mut rules);
    }
    rules
}

fn normalize_line_clamp(
    changed: &mut Styles,
    node: &Node,
    css_rules: &super::super::authored_css::Index<'_>,
) {
    let captured = node
        .style
        .get("-webkit-line-clamp")
        .is_some_and(|value| value != "none" && value != "0");
    let vertical = node
        .style
        .get("-webkit-box-orient")
        .is_some_and(|value| value == "vertical")
        || super::super::authored_css::has_property_indexed(node, css_rules, "-webkit-box-orient");
    let authored = multiline_text_box(node)
        .then(|| {
            super::super::authored_css::positive_integer_property_indexed(
                node,
                css_rules,
                "-webkit-line-clamp",
            )
        })
        .flatten();
    if captured && vertical || authored.is_some() {
        changed.insert("display".into(), "-webkit-box".into());
        if let Some(lines) = authored {
            changed.insert("-webkit-box-orient".into(), "vertical".into());
            changed.insert("-webkit-line-clamp".into(), lines.to_string());
        }
    }
}

fn multiline_text_box(node: &Node) -> bool {
    node.style
        .get("line-height")
        .and_then(|value| value.strip_suffix("px"))
        .and_then(|value| value.parse::<f64>().ok())
        .is_some_and(|line_height| node.rect.height > line_height * 1.5)
}

fn append_pseudo_rule(
    class: &str,
    suffix: &str,
    base: Option<&Pseudo>,
    current: Option<&Pseudo>,
    assets: &BTreeMap<String, String>,
    rules: &mut String,
) {
    let Some(current) = current else {
        if base.is_some() {
            rules.push_str(&format!(".{class}{suffix}{{content:none;}}"));
        }
        return;
    };
    let base_styles = base
        .map(|pseudo| &pseudo.style)
        .cloned()
        .unwrap_or_default();
    let mut changed = changed_styles(&base_styles, &current.style);
    super::super::style_delta::append_reversions(
        &mut changed,
        &super::super::style_delta::declared(&base_styles),
        &current.style,
    );
    // `changed` still carries `content`, so a box whose content varies across viewports
    // declares it twice here — once from the field below and once from the map. Both spell it
    // the same way, so the duplicate is inert. Filtering `content` out of `changed`, as
    // `css_pseudo::declarations` does, would leave the field below as the only source: correct
    // only because that field is now localised.
    let content = if base.is_none_or(|pseudo| pseudo.content != current.content) {
        super::super::css_pseudo::content_declaration(&current.content, assets)
    } else {
        String::new()
    };
    if !content.is_empty() || !changed.is_empty() {
        rules.push_str(&format!(
            ".{class}{suffix}{{{content}{}}}",
            declarations(&changed, assets)
        ));
    }
}

#[cfg(test)]
#[path = "responsive_node_rules_tests.rs"]
mod tests;
