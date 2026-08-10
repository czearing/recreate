use super::{
    generated_source::generated_class,
    source_item_name_words::{camel, month_date, pascal},
};
use std::collections::HashMap;

pub fn collection_entity(sources: &[&mut String]) -> Option<String> {
    let mut counts = HashMap::<String, usize>::new();
    for source in sources {
        let mut remaining = source.as_str();
        while let Some(index) = remaining.find("data-testid={\"") {
            remaining = &remaining[index + "data-testid={\"".len()..];
            let Some(end) = remaining.find("\"}") else {
                break;
            };
            if let Some(entity) = remaining[..end].strip_suffix("-card") {
                *counts.entry(pascal(entity)).or_default() += 1;
            }
            remaining = &remaining[end + 2..];
        }
    }
    counts
        .into_iter()
        .max_by(|(left_name, left_count), (right_name, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| right_name.cmp(left_name))
        })
        .map(|(name, _)| name)
}

pub fn item_name(signature: &str, values: &[String], entity: Option<&str>) -> String {
    if let Some(test_id) = literal_attribute(signature, "data-testid") {
        return pascal(test_id);
    }
    let entity = entity.unwrap_or("Collection");
    let value_text = values
        .iter()
        .filter_map(|value| serde_json::from_str::<String>(value).ok())
        .collect::<Vec<_>>()
        .join(" ");
    if value_text
        .split_whitespace()
        .any(|value| value == format!("{}-card", entity.to_lowercase()))
    {
        return format!("{entity}Card");
    }
    if signature.starts_with("<button") {
        return format!("{entity}ActionButton");
    }
    if signature.starts_with(&format!("<{entity}")) {
        return format!("{entity}ItemContent");
    }
    format!("{entity}Item")
}

pub fn prop_fields(signature: &str, values: &[String], varying: &[usize]) -> Vec<usize> {
    (0..values.len())
        .filter(|index| varying.contains(index) || semantic_field(signature, values, *index))
        .collect()
}

pub fn prop_names(signature: &str, values: &[String], varying: &[usize]) -> Vec<(usize, String)> {
    let mut counts = HashMap::<String, usize>::new();
    varying
        .iter()
        .map(|index| {
            let base = prop_name(signature, values, *index);
            let count = counts.entry(base.clone()).or_default();
            *count += 1;
            let name = if *count == 1 {
                base
            } else {
                format!("{base}{}", *count)
            };
            (*index, name)
        })
        .collect()
}

fn prop_name(signature: &str, values: &[String], index: usize) -> String {
    let marker = format!("{{{{FIELD{index}}}}}");
    let before = signature
        .find(&marker)
        .map_or("", |position| &signature[..position]);
    let value = values
        .get(index)
        .and_then(|value| serde_json::from_str::<String>(value).ok())
        .unwrap_or_default();
    if let Some(attribute) = attribute_before(before) {
        return match attribute {
            "className" => format!("{}ClassName", element_role(before)),
            "aria-label" if element_role(before) == "container" => "ariaLabel".into(),
            "aria-label" => "label".into(),
            "src" => "imageSrc".into(),
            "id" if element_role(before) == "button" => "menuId".into(),
            "id" => "elementId".into(),
            "data-recreate-trigger" => "trigger".into(),
            name => camel(name),
        };
    }

    let role = element_role(before);
    if role.contains("Title") {
        "title".into()
    } else if role.contains("Icon") {
        "icon".into()
    } else if value.starts_with("/assets/") {
        "imageSrc".into()
    } else if value.starts_with("menu") {
        "menuId".into()
    } else if value.parse::<f64>().is_ok() {
        "count".into()
    } else if value.contains(" ago") || month_date(&value) {
        "updatedTime".into()
    } else if value.chars().count() <= 3 && !value.chars().any(char::is_alphanumeric) {
        "icon".into()
    } else {
        "text".into()
    }
}

fn semantic_field(signature: &str, values: &[String], index: usize) -> bool {
    let marker = format!("{{{{FIELD{index}}}}}");
    let before = signature
        .find(&marker)
        .map_or("", |position| &signature[..position]);
    let value = values
        .get(index)
        .and_then(|value| serde_json::from_str::<String>(value).ok())
        .unwrap_or_default();
    if let Some(attribute) = attribute_before(before) {
        return matches!(
            attribute,
            "aria-label" | "src" | "id" | "data-recreate-trigger"
        ) && !value.is_empty();
    }
    !value.is_empty()
        && !generated_class(&value)
        && !matches!(value.as_str(), "button" | "true" | "false")
}

fn attribute_before(source: &str) -> Option<&str> {
    if source.rfind('<')? < source.rfind('>').unwrap_or_default() {
        return None;
    }
    let equal = source.rfind('=')?;
    source[..equal]
        .split(|character: char| character.is_whitespace() || character == '<')
        .next_back()
}

fn element_role(source: &str) -> String {
    let tag = source
        .rfind('<')
        .and_then(|start| {
            source[start + 1..]
                .split(|character: char| character.is_whitespace() || character == '>')
                .next()
        })
        .unwrap_or("element");
    match tag {
        "div" => "container".into(),
        "img" => "image".into(),
        "p" => "title".into(),
        "span" => "label".into(),
        name => camel(name),
    }
}

fn literal_attribute<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}={{\"");
    let start = source.find(&marker)? + marker.len();
    let end = source[start..].find("\"}")? + start;
    Some(&source[start..end])
}
