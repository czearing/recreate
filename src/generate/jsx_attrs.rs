use super::{jsx_attr_names, jsx_host_props, jsx_promotion, stand_in};
use crate::model::Node;
use std::collections::BTreeMap;

/// Every attribute React can honour on the element that carried it. `class` and `style` are
/// emitted from the class map instead, `on*` handlers are re-bound by the runtime, and a
/// relocated attribute is emitted by an ancestor rather than here. What the element is
/// replaced by, and what that replacement may still assert, is `stand_in`'s question.
pub fn attributes(node: &Node, assets: &BTreeMap<String, String>) -> String {
    let source = stand_in::painted_source(node, assets);
    let carried = node
        .attributes
        .iter()
        .filter(|(key, _)| !["class", "style"].contains(&key.as_str()))
        .filter(|(key, _)| !key.starts_with("on"))
        .filter(|(key, _)| !jsx_host_props::relocated(node, key))
        .filter(|(key, _)| !stand_in::suppressed(key, source))
        .map(|(key, value)| render_attribute(key, &crate::asset_attributes::rewrite(value, assets)))
        .collect::<String>();
    format!(
        "{carried}{}{}",
        jsx_promotion::promotion(node),
        stand_in::rendered(node, source)
    )
}

fn render_attribute(key: &str, value: &str) -> String {
    let value = if boolean_attribute(key) {
        "{true}".into()
    } else {
        quoted(value)
    };
    format!(" {}={value}", jsx_attr_names::jsx_attribute(key))
}

fn boolean_attribute(key: &str) -> bool {
    matches!(
        key,
        "allowfullscreen"
            | "autofocus"
            | "autoplay"
            | "checked"
            | "controls"
            | "default"
            | "disabled"
            | "formnovalidate"
            | "hidden"
            | "itemscope"
            | "loop"
            | "multiple"
            | "muted"
            | "novalidate"
            | "open"
            | "playsinline"
            | "readonly"
            | "required"
            | "reversed"
            | "selected"
    )
}

pub fn quoted(value: &str) -> String {
    format!("{{{}}}", serde_json::to_string(value).unwrap())
}

pub fn jsx_tag(value: &str) -> &str {
    match value {
        "lineargradient" => "linearGradient",
        "radialgradient" => "radialGradient",
        "clippath" => "clipPath",
        "textpath" => "textPath",
        "foreignobject" => "foreignObject",
        _ => value,
    }
}

pub fn void_tag(tag: &str) -> bool {
    matches!(
        tag,
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "source"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A node carrying one attribute. Every case here varies only the tag and the
    /// attributes, so the rest of the shape is written once rather than per test.
    fn node(tag: &str, attributes: &[(&str, &str)]) -> Node {
        Node {
            tag: tag.into(),
            attributes: attributes
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            ..Default::default()
        }
    }

    /// Element names, unlike attribute names, are matched case-insensitively by the DOM but
    /// must be spelled exactly for JSX to emit the SVG element rather than an unknown tag.
    #[test]
    fn converts_svg_element_names_to_jsx() {
        assert_eq!(jsx_tag("lineargradient"), "linearGradient");
        assert_eq!(jsx_tag("foreignobject"), "foreignObject");
        assert_eq!(jsx_tag("div"), "div");
    }

    #[test]
    fn keeps_accessibility_state_on_component_instances() {
        let node = node("button", &[("aria-expanded", "true"), ("role", "button")]);
        let output = attributes(&node, &Default::default());
        assert!(output.contains("aria-expanded={\"true\"}"));
        assert!(output.contains("role={\"button\"}"));
    }

    /// A namespaced attribute reaching this emitter is the case that put a colon in an
    /// attribute position. Babel rejects that outright and esbuild lowers it to a string-keyed
    /// prop, which React DOM writes to no namespace at all. Asserted here as well as over the
    /// name translation, because the cheapest wrong repair is a filter in this function that
    /// drops colon-bearing names — which builds cleanly while the sprite reference is gone.
    /// Comparing the whole emitted string is what that repair cannot satisfy.
    #[test]
    fn emits_a_namespaced_attribute_as_the_react_prop_with_its_value_intact() {
        let node = node("use", &[("xlink:href", "#i")]);
        assert_eq!(
            attributes(&node, &Default::default()),
            " xlinkHref={\"#i\"}"
        );
    }

    #[test]
    fn preserves_boolean_control_state() {
        let node = node("button", &[("disabled", "")]);
        assert!(attributes(&node, &Default::default()).contains(" disabled={true}"));
    }

    /// The same `boolean_attribute` arm that keeps `disabled` must drop `selected`, because
    /// React reads selection from the parent `<select>` and ignores the prop entirely.
    #[test]
    fn drops_control_state_the_host_expresses_elsewhere() {
        let node = node("option", &[("selected", ""), ("value", "gold")]);
        let output = attributes(&node, &Default::default());
        assert!(!output.contains("selected"));
        assert!(output.contains(" value={\"gold\"}"));
    }
}
