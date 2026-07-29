use super::authored_css_rules::declarations;
use crate::model::{Node, Styles};

pub fn normalize(styles: &mut Styles, node: &Node, rules: &[String]) {
    let mut authored = declarations(node, rules);
    if node.tag == "textarea"
        && authored
            .get("height")
            .is_some_and(|value| value.ends_with('%'))
    {
        authored.remove("height");
    }

    if authored.is_empty() {
        return;
    }
    for property in ["animation", "transition"] {
        if authored.contains_key(property) {
            styles.remove(property);
        }
    }
    let intrinsic_reveal = authored
        .get("max-width")
        .is_some_and(|value| value == "0" || value == "0px")
        && authored
            .get("transition")
            .is_some_and(|value| value.contains("max-width"));
    if intrinsic_reveal && !authored.contains_key("width") {
        styles.remove("width");
    }
    let centered = authored
        .get("margin")
        .is_some_and(|value| value.split_whitespace().any(|part| part == "auto"));
    if centered {
        if authored.contains_key("max-width") && !authored.contains_key("width") {
            styles.remove("width");
        }
        styles.insert("margin-left".into(), "auto".into());
        styles.insert("margin-right".into(), "auto".into());
    }
    if !authored.contains_key("width") && flexible(&authored) {
        styles.remove("width");
    }
    if node.tag != "textarea" && !authored.contains_key("height") && flexible(&authored) {
        styles.remove("height");
    }
    if matches!(
        authored.get("display").map(String::as_str),
        Some("grid" | "inline-grid")
    ) && !authored.contains_key("grid-template-rows")
    {
        styles.remove("grid-template-rows");
    }
    if !authored.contains_key("height")
        && authored.contains_key("min-height")
        && matches!(
            authored.get("display").map(String::as_str),
            Some("flex" | "inline-flex" | "grid" | "inline-grid")
        )
    {
        styles.remove("height");
    }
    styles.extend(authored);
}

pub fn has_property(node: &Node, rules: &[String], property: &str) -> bool {
    rules
        .iter()
        .filter_map(|rule| rule.split_once('{'))
        .any(|(selector, declarations)| {
            !selector.starts_with('@')
                && !selector.contains(':')
                && directly_targets_node(selector, node)
                && declarations
                    .split(';')
                    .filter_map(|declaration| declaration.split_once(':'))
                    .any(|(name, _)| name.trim() == property)
        })
}

pub fn positive_integer_property(node: &Node, rules: &[String], property: &str) -> Option<u32> {
    rules
        .iter()
        .filter_map(|rule| rule.split_once('{'))
        .filter(|(selector, _)| {
            !selector.starts_with('@')
                && !selector.contains(':')
                && directly_targets_node(selector, node)
        })
        .flat_map(|(_, declarations)| declarations.split(';'))
        .filter_map(|declaration| declaration.split_once(':'))
        .filter(|(name, _)| name.trim() == property)
        .map(|(_, value)| {
            value
                .trim()
                .trim_end_matches('}')
                .trim()
                .trim_end_matches("!important")
                .trim()
        })
        .next_back()?
        .parse()
        .ok()
        .filter(|value| *value > 0)
}

fn flexible(styles: &Styles) -> bool {
    if styles
        .get("flex-grow")
        .and_then(|value| value.parse::<f64>().ok())
        .is_some_and(|value| value > 0.0)
    {
        return true;
    }

    styles
        .get("flex")
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse::<f64>().ok())
        .is_some_and(|value| value > 0.0)
}

pub(super) use super::authored_css_rules::directly_targets_node;

#[cfg(test)]
#[path = "authored_css_tests_1.rs"]
mod tests_1;
#[cfg(test)]
#[path = "authored_css_tests_2.rs"]
mod tests_2;
#[cfg(test)]
#[path = "authored_css_tests_3.rs"]
mod tests_3;
