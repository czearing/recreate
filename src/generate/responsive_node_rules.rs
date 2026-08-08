use super::flex::{fluid_flex_item, shrunk_flex_item};
use crate::{
    generate::css::declarations,
    model::{Node, Pseudo, Styles, Viewport},
};
use std::collections::BTreeMap;

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(super) fn append_node_rules(
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
    if !changed.is_empty() {
        rules.push_str(&format!(".{class}{{{}}}", declarations(&changed, assets)));
    }
    append_pseudo_rule(
        class,
        "before",
        base.before.as_ref(),
        node.before.as_ref(),
        assets,
        &mut rules,
    );
    append_pseudo_rule(
        class,
        "after",
        base.after.as_ref(),
        node.after.as_ref(),
        assets,
        &mut rules,
    );
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
    name: &str,
    base: Option<&Pseudo>,
    current: Option<&Pseudo>,
    assets: &BTreeMap<String, String>,
    rules: &mut String,
) {
    let Some(current) = current else {
        if base.is_some() {
            rules.push_str(&format!(".{class}::{name}{{content:none;}}"));
        }
        return;
    };
    let base_styles = base
        .map(|pseudo| &pseudo.style)
        .cloned()
        .unwrap_or_default();
    let changed = changed_styles(&base_styles, &current.style);
    let content = if base.is_none_or(|pseudo| pseudo.content != current.content) {
        format!("content:{};", current.content)
    } else {
        String::new()
    };
    if !content.is_empty() || !changed.is_empty() {
        rules.push_str(&format!(
            ".{class}::{name}{{{content}{}}}",
            declarations(&changed, assets)
        ));
    }
}

pub(in crate::generate) fn changed_styles(base: &Styles, current: &Styles) -> Styles {
    current
        .iter()
        .filter(|(key, value)| base.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Rect;

    fn heading(font_size: &str, width: f64) -> Node {
        Node {
            disabled: false,
            path: "html>body>h1".into(),
            parent: Some("html>body".into()),
            tag: "h1".into(),
            text: "Heading".into(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width,
                height: 100.0,
            },
            style: Styles::from([
                ("font-size".into(), font_size.into()),
                ("width".into(), format!("{width}px")),
            ]),
            before: None,
            after: None,
        }
    }

    #[test]
    fn emits_responsive_clamped_typography() {
        let base = heading("68px", 760.0);
        let current = heading("53.76px", 603.0);
        let css = append_node_rules(
            &base,
            &current,
            None,
            (
                &Viewport {
                    width: 1920,
                    height: 1080,
                    dpr: 1.0,
                },
                &Viewport {
                    width: 768,
                    height: 1024,
                    dpr: 1.0,
                },
            ),
            "heading",
            &Default::default(),
            &["h1{font-size:clamp(36px,7vw,68px);max-width:16ch;}".into()],
            false,
            false,
        );
        assert!(css.contains("font-size:53.76px"), "{css}");
    }

    fn shrinking_title(width: f64) -> Node {
        Node {
            disabled: false,
            path: "html>body>div>span".into(),
            parent: Some("html>body>div".into()),
            tag: "span".into(),
            text: "Untitled notebook".into(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width,
                height: 22.0,
            },
            style: Styles::from([
                ("flex-shrink".into(), "1".into()),
                ("width".into(), format!("{width}px")),
            ]),
            before: None,
            after: None,
        }
    }

    fn shrinking_title_row(width: f64) -> Node {
        let mut parent = shrinking_title(width);
        parent.path = "html>body>div".into();
        parent.tag = "div".into();
        parent.style = Styles::from([
            ("display".into(), "flex".into()),
            ("flex-direction".into(), "row".into()),
        ]);
        parent
    }

    fn shrunk_title_rule() -> String {
        append_node_rules(
            &shrinking_title(234.0),
            &shrinking_title(170.65625),
            Some(&shrinking_title_row(420.0)),
            (
                &Viewport {
                    width: 1440,
                    height: 900,
                    dpr: 1.0,
                },
                &Viewport {
                    width: 768,
                    height: 1024,
                    dpr: 1.0,
                },
            ),
            "title",
            &Default::default(),
            &[],
            false,
            false,
        )
    }

    #[test]
    fn lets_a_shrunk_flex_item_shrink_instead_of_pinning_its_sampled_width() {
        let css = shrunk_title_rule();
        assert!(css.contains("min-width:0"), "{css}");
    }

    #[test]
    fn never_freezes_a_sampled_pixel_width_on_a_shrunk_flex_item() {
        let css = shrunk_title_rule();
        assert!(!css.contains("max-width:170.65625px"), "{css}");
        assert!(!css.contains("width:170.65625px"), "{css}");
    }
}
