use super::{jsx_attr_names, jsx_host_props};
use crate::model::Node;
use std::collections::BTreeMap;

/// Every attribute React can honour on the element that carried it. `class` and `style` are
/// emitted from the class map instead, `on*` handlers are re-bound by the runtime, and a
/// relocated attribute is emitted by an ancestor rather than here.
pub fn attributes(node: &Node, assets: &BTreeMap<String, String>) -> String {
    node.attributes
        .iter()
        .filter(|(key, _)| !["class", "style"].contains(&key.as_str()))
        .filter(|(key, _)| !key.starts_with("on"))
        .filter(|(key, _)| !jsx_host_props::relocated(&node.tag, key))
        .map(|(key, value)| render_attribute(key, rewrite(value, assets)))
        .collect()
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

fn rewrite<'a>(value: &'a str, assets: &'a BTreeMap<String, String>) -> &'a str {
    assets.get(value).map(String::as_str).unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut node = crate::model::Node {
            path: "html>body:nth-of-type(1)>button:nth-of-type(1)".into(),
            parent: Some("html>body:nth-of-type(1)".into()),
            tag: "button".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: crate::model::Rect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            style: Default::default(),
            before: None,
            after: None,
        };
        node.attributes
            .insert("aria-expanded".into(), "true".into());
        node.attributes.insert("role".into(), "button".into());
        let output = attributes(&node, &Default::default());
        assert!(output.contains("aria-expanded={\"true\"}"));
        assert!(output.contains("role={\"button\"}"));
    }

    #[test]
    fn preserves_boolean_control_state() {
        let mut node = crate::model::Node {
            path: "html>body:nth-of-type(1)>button:nth-of-type(1)".into(),
            parent: Some("html>body:nth-of-type(1)".into()),
            tag: "button".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: crate::model::Rect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            style: Default::default(),
            before: None,
            after: None,
        };
        node.attributes.insert("disabled".into(), String::new());
        assert!(attributes(&node, &Default::default()).contains(" disabled={true}"));
    }

    /// The same `boolean_attribute` arm that keeps `disabled` must drop `selected`, because
    /// React reads selection from the parent `<select>` and ignores the prop entirely.
    #[test]
    fn drops_control_state_the_host_expresses_elsewhere() {
        let mut node = crate::model::Node {
            path: "html>body:nth-of-type(1)>select:nth-of-type(1)>option:nth-of-type(1)".into(),
            parent: Some("html>body:nth-of-type(1)>select:nth-of-type(1)".into()),
            tag: "option".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: crate::model::Rect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            style: Default::default(),
            before: None,
            after: None,
        };
        node.attributes.insert("selected".into(), String::new());
        node.attributes.insert("value".into(), "gold".into());
        let output = attributes(&node, &Default::default());
        assert!(!output.contains("selected"));
        assert!(output.contains(" value={\"gold\"}"));
    }
}
