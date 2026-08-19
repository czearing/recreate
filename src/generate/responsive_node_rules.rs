use super::super::style_delta::changed_styles;
use super::flex::{fluid_flex_item, shrunk_flex_item};
use crate::{
    generate::css::declarations,
    model::{Node, Styles, Viewport},
};
use std::collections::BTreeMap;

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(in crate::generate) fn append_node_rules<'a>(
    base: &Node,
    node: &Node,
    parent: Option<&Node>,
    viewports: (&Viewport, &Viewport),
    class: &str,
    assets: &BTreeMap<String, String>,
    css_rules: impl Into<super::super::authored_css::Authored<'a>>,
    fluid_height: bool,
    constrained_by_flex: bool,
) -> String {
    let index = super::super::authored_css::Index::new(css_rules);
    let parts = node_rule_parts(
        base,
        node,
        parent,
        viewports,
        assets,
        &index,
        fluid_height,
        constrained_by_flex,
    );
    parts_text(&[class], &parts)
}

/// Writes a rule set under one or more classes. Two elements whose band declarations are
/// identical are one rule with a selector list, not two rules that happen to say the same
/// thing: the band is where a shared body is visible, so it is where the sharing is spelled.
pub(super) fn parts_text(classes: &[&str], parts: &[(String, String)]) -> String {
    let mut rules = String::new();
    for (suffix, body) in parts {
        let selectors = classes
            .iter()
            .map(|class| format!(".{class}{suffix}"))
            .collect::<Vec<_>>()
            .join(",");
        rules.push_str(&format!("{selectors}{{{body}}}"));
    }
    rules
}

/// The declarations an element needs at this viewport, as a suffix and a body per rule. Kept
/// apart from the class so the caller can decide how many elements one body serves.
#[allow(clippy::too_many_arguments)]
pub(super) fn node_rule_parts(
    base: &Node,
    node: &Node,
    parent: Option<&Node>,
    viewports: (&Viewport, &Viewport),
    assets: &BTreeMap<String, String>,
    css_rules: &super::super::authored_css::Index<'_>,
    fluid_height: bool,
    constrained_by_flex: bool,
) -> Vec<(String, String)> {
    let mut rules = Vec::new();
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
    super::super::style_delta::remove_restatements(
        &mut changed,
        &normalized_base(base, parent, css_rules),
    );
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
        rules.push((String::new(), declarations(&changed, assets)));
    }
    for (suffix, base, current) in super::super::css_pseudo::paired(&base.pseudos, &node.pseudos) {
        super::pseudo_rules::append_pseudo_rule(suffix, base, current, assets, &mut rules);
    }
    rules
}

/// The base viewport's rule as the same normalizers would write it, which is what a band's
/// declarations have to be a difference from. Only the viewport-independent normalizers belong
/// here: a geometry normalizer already answers a question about this band alone.
fn normalized_base(
    base: &Node,
    parent: Option<&Node>,
    css_rules: &super::super::authored_css::Index<'_>,
) -> Styles {
    let mut styles = base.style.clone();
    super::super::authored_css::normalize_indexed(&mut styles, base, css_rules);
    super::super::inherited_styles::normalize_indexed(&mut styles, base, parent, css_rules);
    styles
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

#[cfg(test)]
#[path = "responsive_node_rules_tests.rs"]
mod tests;
