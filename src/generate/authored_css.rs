use crate::model::{Node, Styles};

pub use super::authored_css_index::Index;

pub fn normalize(styles: &mut Styles, node: &Node, rules: &[String]) {
    normalize_indexed(styles, node, &Index::new(rules));
}

pub fn normalize_indexed(styles: &mut Styles, node: &Node, rules: &Index<'_>) {
    let mut authored = rules.declarations(node);
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
    // A box the author never gave a height to is sized by its content. Keeping
    // the sampled pixel height freezes the line and row count observed at the
    // capture viewport, so text or a row that wraps at a narrower width
    // overflows the frozen box instead of growing it, and everything below it
    // moves up. Two shapes qualify: a flex or grid container, and any box with
    // an authored minimum height, which is a floor the author expects content
    // to grow past. The authored map drops custom-property values, so ask the
    // rules directly: a size written as a variable is still one the author meant.
    let authored_height = ["height", "block-size"]
        .iter()
        .any(|property| rules.has_property(node, property));
    let content_sized = matches!(
        authored.get("display").map(String::as_str),
        Some("flex" | "inline-flex" | "grid" | "inline-grid")
    ) || ["min-height", "min-block-size"]
        .iter()
        .any(|property| rules.has_property(node, property));
    if !authored_height && content_sized {
        styles.remove("height");
    }
    styles.extend(authored);
}

pub fn has_property(node: &Node, rules: &[String], property: &str) -> bool {
    has_property_indexed(node, &Index::new(rules), property)
}

pub fn has_property_indexed(node: &Node, rules: &Index<'_>, property: &str) -> bool {
    rules.has_property(node, property)
}

#[cfg(test)]
pub fn positive_integer_property(node: &Node, rules: &[String], property: &str) -> Option<u32> {
    positive_integer_property_indexed(node, &Index::new(rules), property)
}

pub fn positive_integer_property_indexed(
    node: &Node,
    rules: &Index<'_>,
    property: &str,
) -> Option<u32> {
    rules.positive_integer_property(node, property)
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
