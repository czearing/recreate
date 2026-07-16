use crate::model::Node;
use std::collections::BTreeMap;

pub fn all_attributes(node: &Node, assets: &BTreeMap<String, String>) -> String {
    node.attributes
        .iter()
        .filter(|(key, _)| !["class", "style"].contains(&key.as_str()))
        .filter(|(key, _)| !key.starts_with("on"))
        .map(|(key, value)| format!(" {}={}", jsx_attribute(key), quoted(rewrite(value, assets))))
        .collect()
}

pub fn static_attributes(node: &Node, assets: &BTreeMap<String, String>) -> String {
    node.attributes
        .iter()
        .filter(|(key, _)| {
            !matches!(
                key.as_str(),
                "class"
                    | "style"
                    | "id"
                    | "href"
                    | "src"
                    | "srcset"
                    | "alt"
                    | "title"
                    | "aria-label"
                    | "value"
            )
        })
        .map(|(key, value)| format!(" {}={}", jsx_attribute(key), quoted(rewrite(value, assets))))
        .collect()
}

pub fn dynamic_attributes(node: &Node, assets: &BTreeMap<String, String>) -> String {
    node.attributes
        .iter()
        .filter(|(key, _)| {
            matches!(
                key.as_str(),
                "id" | "href" | "src" | "srcset" | "alt" | "title" | "aria-label" | "value"
            )
        })
        .map(|(key, value)| format!(" {}={}", jsx_attribute(key), quoted(rewrite(value, assets))))
        .collect()
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
}
