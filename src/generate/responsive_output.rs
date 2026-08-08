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
) -> String {
    base_declarations_indexed(
        node,
        parent,
        viewport,
        assets,
        &super::super::authored_css::Index::new(css_rules),
        fluid_height,
    )
}

pub fn base_declarations_indexed(
    node: &Node,
    parent: Option<&Node>,
    viewport: &Viewport,
    assets: &BTreeMap<String, String>,
    css_rules: &super::super::authored_css::Index<'_>,
    fluid_height: bool,
) -> String {
    let mut styles = node.style.clone();
    super::super::authored_css::normalize_indexed(&mut styles, node, css_rules);
    if fluid_height {
        styles.remove("height");
    }
    super::super::inherited_styles::normalize_indexed(&mut styles, node, parent, css_rules);
    super::super::responsive_geometry::normalize(&mut styles, node, parent, viewport, None);
    // Last, so that nothing downstream can put a sampled pixel back.
    remove_sampled_sizes(&mut styles, node, css_rules);
    normalize(&mut styles);
    declarations(&styles, assets)
}

/// The size properties, in the two axes plus the shorthands that resolve to them.
const SIZE_PROPERTIES: [&str; 9] = [
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    "flex-basis",
    "grid-template-columns",
    "grid-template-rows",
];

/// A captured style is a *used* value: `getComputedStyle` resolves `width: 50%`,
/// `flex: 1`, `clamp()`, `var()`, and `grid-template-columns: 1fr 2fr` to the pixels they
/// happened to occupy at the captured viewport. Emitting that pixel back out produces a
/// page that is correct at exactly one width and wrong everywhere else, and no amount of
/// per-tag guessing at "is this pixel real?" can recover the authored intent, because the
/// intent was destroyed before the emitter ever saw the value.
///
/// So the emitter never invents a size. If the source authored a size, the authored value
/// is used verbatim; otherwise nothing is emitted and the box is sized by the same flow
/// that sized it in the source. Replaced elements are the one exception, and not a
/// per-site one: they have no in-flow content to reflow, and their box must be reserved
/// or the page shifts as they load.
fn remove_sampled_sizes(
    styles: &mut Styles,
    node: &Node,
    css_rules: &super::super::authored_css::Index<'_>,
) {
    if is_replaced(node) {
        return;
    }
    for property in SIZE_PROPERTIES {
        if !styles
            .get(property)
            .is_some_and(|value| value.ends_with("px"))
        {
            continue;
        }
        match css_rules.authored_value(node, property) {
            Some(authored) => styles.insert(property.into(), authored),
            None => styles.remove(property),
        };
    }
}

/// Replaced elements are sized by their own intrinsic content rather than by the flow.
fn is_replaced(node: &Node) -> bool {
    matches!(
        node.tag.as_str(),
        "img" | "svg" | "video" | "canvas" | "iframe" | "embed" | "object"
    )
}

/// The capture records a declaration only where its value differs from what the element
/// would compute with no author CSS, so every declaration that arrives here is already
/// load-bearing. What remains is removing declarations that are inert in combination
/// with another one, which is a statement about pairs of values rather than about names.
pub fn output_declarations(styles: &Styles, assets: &BTreeMap<String, String>) -> String {
    let mut styles = styles.clone();
    normalize(&mut styles);
    declarations(&styles, assets)
}

fn normalize(styles: &mut Styles) {
    remove_overridden_shorthands(styles);
    remove_inert_origins(styles);
    remove_static_insets(styles);
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

/// An inset only does something on a positioned box, so on a static one the sampled
/// `auto` is noise. On a positioned box it is load-bearing: it is what stops an
/// authored offset from applying on the other axis.
/// A shorthand the authored index contributed is inert once the longhands that spell it
/// out are present, because sorted emission places them after it.
fn remove_overridden_shorthands(styles: &mut Styles) {
    let inert: Vec<String> = styles
        .keys()
        .filter(|name| {
            crate::generate::authored_css_rules::overridden_shorthand(name, |part| {
                styles.contains_key(part)
            })
        })
        .cloned()
        .collect();
    for name in inert {
        styles.remove(&name);
    }
}

/// An origin names where a transform or a perspective is anchored. With neither
/// declared it anchors nothing, and its resolved value is just the box centre restated
/// in pixels — which the recreation recomputes from the box it already reproduces. The
/// test is what the CSS says has an effect, not which property it is.
fn remove_inert_origins(styles: &mut Styles) {
    let transformed = ["transform", "rotate", "scale", "translate", "offset-path"]
        .iter()
        .any(|name| styles.contains_key(*name))
        || styles.contains_key("animation-name");
    if !transformed {
        styles.remove("transform-origin");
    }
    if !styles.contains_key("perspective") {
        styles.remove("perspective-origin");
    }
}

fn remove_static_insets(styles: &mut Styles) {
    if styles.get("position").is_none_or(|value| value == "static") {
        for side in ["top", "right", "bottom", "left"] {
            if styles.get(side).is_some_and(|value| value == "auto") {
                styles.remove(side);
            }
        }
    }
}
#[cfg(test)]
mod scrollbar_output_tests {
    use super::output_declarations;
    use crate::model::Styles;
    use std::collections::BTreeMap;

    fn render(name: &str, value: &str) -> String {
        let mut styles = Styles::new();
        styles.insert(name.into(), value.into());
        output_declarations(&styles, &BTreeMap::new())
    }

    /// A thin scrollbar is 10px where the default is 15px, so dropping it
    /// makes every scroll container 5px narrower than the source.
    #[test]
    fn a_thin_scrollbar_reaches_the_stylesheet() {
        assert!(render("scrollbar-width", "thin").contains("scrollbar-width"));
        assert!(render("scrollbar-gutter", "stable").contains("scrollbar-gutter"));
    }

    /// The companion assertion - that `auto` is dropped - was removed because it was
    /// wrong, not because it was inconvenient. `scrollbar-color` is inherited, so
    /// `auto` inside a container that set a colour is an override, and deleting it by
    /// value would restore the container's colour on a child that asked for the
    /// default. Whether a value is redundant depends on the element's surroundings,
    /// which the emitter cannot see; the capture answers it by measurement instead.
    #[test]
    fn a_default_scrollbar_declaration_is_not_deleted_by_value() {
        assert!(render("scrollbar-color", "auto").contains("scrollbar-color:auto"));
        assert!(render("scrollbar-width", "auto").contains("scrollbar-width:auto"));
    }
}
