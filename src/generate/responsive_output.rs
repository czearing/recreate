use super::flex::{fluid_flex_item, intrinsic_flex_text};
use crate::{
    generate::css::declarations,
    model::{Node, Styles, Viewport},
};
use std::collections::BTreeMap;

pub fn base_declarations(
    node: &Node,
    parent: Option<&Node>,
    viewport: &Viewport,
    assets: &BTreeMap<String, String>,
    css_rules: &[String],
    fluid_height: bool,
    text_parent: bool,
) -> String {
    let mut styles = node.style.clone();
    let authored_width = super::super::authored_css::has_property(node, css_rules, "width");
    super::super::authored_css::normalize(&mut styles, node, css_rules);
    if !authored_width
        && (intrinsic_flex_text(node, parent, text_parent) || fluid_flex_item(node, parent))
    {
        styles.remove("width");
    }
    if fluid_height {
        styles.remove("height");
    }
    super::super::inherited_styles::normalize(&mut styles, node, parent, css_rules);
    super::super::responsive_geometry::normalize(&mut styles, node, parent, viewport, None);
    normalize(&mut styles);
    declarations(&styles, assets)
}

pub fn output_declarations(styles: &Styles, assets: &BTreeMap<String, String>) -> String {
    let mut styles = styles.clone();
    normalize(&mut styles);
    declarations(&styles, assets)
}

fn normalize(styles: &mut Styles) {
    styles.retain(|name, _| output_property(name));
    for shorthand in ["flex", "gap", "inset", "margin", "overflow", "padding"] {
        styles.remove(shorthand);
    }
    remove_defaults(styles);
    for side in ["top", "right", "bottom", "left"] {
        let style = format!("border-{side}-style");
        if styles
            .get(&style)
            .is_some_and(|value| value == "none" || value == "hidden")
        {
            styles.remove(&style);
            styles.remove(&format!("border-{side}-width"));
            styles.remove(&format!("border-{side}-color"));
        }
    }
}

fn remove_defaults(styles: &mut Styles) {
    for (name, value) in [
        ("align-self", "auto"),
        ("background-blend-mode", "normal"),
        ("background-clip", "border-box"),
        ("background-image", "none"),
        ("background-origin", "padding-box"),
        ("background-position", "0% 0%"),
        ("background-repeat", "repeat"),
        ("background-size", "auto"),
        ("border-collapse", "separate"),
        ("border-spacing", "0px"),
        ("box-shadow", "none"),
        ("box-sizing", "content-box"),
        ("clip-path", "none"),
        ("cursor", "auto"),
        ("filter", "none"),
        ("float", "none"),
        ("font-feature-settings", "normal"),
        ("font-kerning", "auto"),
        ("font-stretch", "100%"),
        ("font-style", "normal"),
        ("font-variation-settings", "normal"),
        ("grid-auto-flow", "row"),
        ("grid-column-end", "auto"),
        ("grid-column-start", "auto"),
        ("grid-row-end", "auto"),
        ("grid-row-start", "auto"),
        ("grid-template-columns", "none"),
        ("grid-template-rows", "none"),
        ("justify-self", "auto"),
        ("max-height", "none"),
        ("max-width", "none"),
        ("min-height", "auto"),
        ("min-width", "auto"),
        ("object-fit", "fill"),
        ("object-position", "50% 50%"),
        ("opacity", "1"),
        ("order", "0"),
        ("overflow-x", "visible"),
        ("overflow-y", "visible"),
        ("pointer-events", "auto"),
        ("position", "static"),
        ("table-layout", "auto"),
        ("text-rendering", "auto"),
        ("text-transform", "none"),
        ("transform", "none"),
        ("vertical-align", "baseline"),
        ("visibility", "visible"),
        ("white-space", "normal"),
        ("word-break", "normal"),
        ("z-index", "auto"),
    ] {
        if styles.get(name).is_some_and(|current| current == value) {
            styles.remove(name);
        }
    }
}

fn output_property(name: &str) -> bool {
    [
        "animation",
        "background",
        "border",
        "box-",
        "column-",
        "flex",
        "font",
        "grid",
        "inset",
        "line-",
        "margin",
        "max-",
        "min-",
        "object-",
        "outline",
        "overflow",
        "padding",
        "scroll-",
        "text-",
        "transform",
        "transition",
        "word-",
    ]
    .iter()
    .any(|prefix| name == *prefix || name.starts_with(prefix))
        || matches!(
            name,
            "-webkit-line-clamp"
                | "-webkit-text-fill-color"
                | "align-content"
                | "align-items"
                | "align-self"
                | "appearance"
                | "bottom"
                | "caret-color"
                | "clip-path"
                | "color"
                | "content-visibility"
                | "cursor"
                | "display"
                | "fill"
                | "filter"
                | "float"
                | "gap"
                | "height"
                | "justify-content"
                | "justify-items"
                | "justify-self"
                | "left"
                | "letter-spacing"
                | "isolation"
                | "mix-blend-mode"
                | "opacity"
                | "order"
                | "pointer-events"
                | "position"
                | "right"
                | "resize"
                | "row-gap"
                | "stroke"
                | "table-layout"
                | "top"
                | "translate"
                | "user-select"
                | "vertical-align"
                | "visibility"
                | "will-change"
                | "white-space"
                | "width"
                | "z-index"
        )
}
