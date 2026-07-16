use crate::model::Node;
use std::collections::BTreeMap;

pub fn all_attributes(node: &Node, assets: &BTreeMap<String, String>) -> String {
    node.attributes
        .iter()
        .filter(|(key, _)| !["class", "style"].contains(&key.as_str()))
        .filter(|(key, _)| !key.starts_with("on"))
        .map(|(key, value)| render_attribute(key, rewrite(value, assets)))
        .collect()
}

pub fn static_attributes(_node: &Node, _assets: &BTreeMap<String, String>) -> String {
    String::new()
}

pub fn dynamic_attributes(node: &Node, assets: &BTreeMap<String, String>) -> String {
    node.attributes
        .iter()
        .filter(|(key, _)| !["class", "style"].contains(&key.as_str()))
        .filter(|(key, _)| !key.starts_with("on"))
        .map(|(key, value)| render_attribute(key, rewrite(value, assets)))
        .collect()
}

fn render_attribute(key: &str, value: &str) -> String {
    let value = if boolean_attribute(key) {
        "{true}".into()
    } else {
        quoted(value)
    };
    format!(" {}={value}", jsx_attribute(key))
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

fn jsx_attribute(value: &str) -> String {
    match value {
        "for" => "htmlFor".into(),
        "tabindex" => "tabIndex".into(),
        "readonly" => "readOnly".into(),
        _ if value.starts_with("aria-") || value.starts_with("data-") => value.into(),
        _ => camel(value),
    }
}

fn camel(value: &str) -> String {
    let mut result = String::new();
    let mut uppercase = false;
    for character in value.chars() {
        if character == '-' {
            uppercase = true;
        } else if uppercase {
            result.extend(character.to_uppercase());
            uppercase = false;
        } else {
            result.push(character);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_svg_names_to_jsx() {
        assert_eq!(jsx_attribute("stroke-width"), "strokeWidth");
        assert_eq!(jsx_attribute("aria-label"), "aria-label");
        assert_eq!(jsx_tag("lineargradient"), "linearGradient");
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
        let output = dynamic_attributes(&node, &Default::default());
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
        assert!(all_attributes(&node, &Default::default()).contains(" disabled={true}"));
    }
}
