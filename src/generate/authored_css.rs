use crate::model::{Node, Styles};
use std::collections::BTreeMap;

pub fn normalize(styles: &mut Styles, node: &Node, rules: &[String]) {
    let authored = declarations(node, rules);
    if authored.is_empty() {
        return;
    }
    for property in ["animation", "transition"] {
        styles.remove(property);
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
    let centered_max_width = authored.contains_key("max-width")
        && !authored.contains_key("width")
        && authored
            .get("margin")
            .is_some_and(|value| value.split_whitespace().any(|part| part == "auto"));
    if centered_max_width {
        styles.remove("width");
        styles.insert("margin-left".into(), "auto".into());
        styles.insert("margin-right".into(), "auto".into());
    }
    if !authored.contains_key("width") && flexible(&authored) {
        styles.remove("width");
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
    let classes: Vec<_> = node
        .attributes
        .get("class")
        .into_iter()
        .flat_map(|value| value.split_whitespace())
        .collect();
    rules
        .iter()
        .filter_map(|rule| rule.split_once('{'))
        .any(|(selector, declarations)| {
            !selector.starts_with('@')
                && !selector.contains(':')
                && classes
                    .iter()
                    .any(|class| directly_targets(selector, class))
                && declarations
                    .split(';')
                    .filter_map(|declaration| declaration.split_once(':'))
                    .any(|(name, _)| name.trim() == property)
        })
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

fn declarations(node: &Node, rules: &[String]) -> Styles {
    let classes: Vec<_> = node
        .attributes
        .get("class")
        .into_iter()
        .flat_map(|value| value.split_whitespace())
        .collect();
    let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for declarations in rules.iter().filter_map(|rule| {
        let (selector, declarations) = rule.split_once('{')?;
        (!selector.starts_with('@')
            && !selector.contains(':')
            && classes
                .iter()
                .any(|class| directly_targets(selector, class)))
        .then_some(declarations)
    }) {
        for (name, value) in declarations
            .split(';')
            .filter_map(|declaration| declaration.split_once(':'))
            .map(|(name, value)| (name.trim(), value.trim()))
            .filter(|(name, value)| retained(name) && !value.contains("var("))
        {
            values.entry(name.into()).or_default().push(value.into());
        }
    }
    values
        .into_iter()
        .filter_map(|(name, values)| {
            let first = values.first()?;
            values
                .iter()
                .all(|value| value == first)
                .then(|| (name, first.clone()))
                .filter(|(name, value)| resolved_matches(node, name, value))
        })
        .collect()
}

fn resolved_matches(node: &Node, name: &str, value: &str) -> bool {
    if matches!(name, "width" | "height") && value == "auto" {
        return node
            .style
            .get(name)
            .is_none_or(|computed| computed == value);
    }
    if !matches!(
        name,
        "align-content"
            | "align-items"
            | "align-self"
            | "column-gap"
            | "display"
            | "flex-direction"
            | "flex-flow"
            | "flex-wrap"
            | "gap"
            | "justify-content"
            | "justify-items"
            | "justify-self"
            | "order"
            | "position"
            | "row-gap"
            | "white-space"
    ) {
        return true;
    }
    node.style
        .get(name)
        .is_none_or(|computed| computed == value)
}

fn directly_targets(selectors: &str, class: &str) -> bool {
    selectors.split(',').any(|selector| {
        let compound = selector
            .trim()
            .rsplit(|character: char| {
                character.is_whitespace() || matches!(character, '>' | '+' | '~')
            })
            .find(|part| !part.is_empty())
            .unwrap_or_default();
        let needle = format!(".{class}");
        compound.match_indices(&needle).any(|(index, _)| {
            compound[index + needle.len()..]
                .chars()
                .next()
                .is_none_or(|character| {
                    !character.is_ascii_alphanumeric() && !matches!(character, '-' | '_')
                })
        })
    })
}

fn retained(name: &str) -> bool {
    matches!(
        name,
        "align-content"
            | "align-items"
            | "align-self"
            | "bottom"
            | "box-sizing"
            | "column-gap"
            | "display"
            | "flex"
            | "flex-basis"
            | "flex-direction"
            | "flex-flow"
            | "flex-grow"
            | "flex-shrink"
            | "flex-wrap"
            | "gap"
            | "grid-auto-columns"
            | "grid-auto-flow"
            | "grid-auto-rows"
            | "grid-column"
            | "grid-row"
            | "grid-template-columns"
            | "grid-template-rows"
            | "height"
            | "inset"
            | "justify-content"
            | "justify-items"
            | "justify-self"
            | "left"
            | "margin"
            | "margin-bottom"
            | "margin-left"
            | "margin-right"
            | "margin-top"
            | "max-height"
            | "max-width"
            | "min-height"
            | "min-width"
            | "object-fit"
            | "opacity"
            | "order"
            | "overflow"
            | "overflow-x"
            | "overflow-y"
            | "padding"
            | "padding-bottom"
            | "padding-left"
            | "padding-right"
            | "padding-top"
            | "position"
            | "right"
            | "row-gap"
            | "top"
            | "transform"
            | "transform-origin"
            | "transition"
            | "translate"
            | "white-space"
            | "width"
            | "z-index"
    )
}

#[cfg(test)]
mod tests {
    use super::normalize;
    use crate::model::{Node, Rect, Styles};

    #[test]
    fn restores_authored_intrinsic_motion() {
        let mut node = Node {
            path: "button".into(),
            parent: None,
            tag: "button".into(),
            text: "Create".into(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 2.0,
                height: 28.0,
            },
            style: Styles::from([
                ("width".into(), "2px".into()),
                ("height".into(), "28px".into()),
                ("transition".into(), "all".into()),
            ]),
            before: None,
            after: None,
        };
        node.attributes.insert("class".into(), "create".into());
        let captured = node.clone();
        normalize(
            &mut node.style,
            &captured,
            &[".create { max-width: 0; transition: opacity .2s, max-width .3s; }".into()],
        );
        assert!(!node.style.contains_key("width"));
        assert_eq!(node.style["height"], "28px");
        assert_eq!(node.style["transition"], "opacity .2s, max-width .3s");
        assert_eq!(node.style["max-width"], "0");
    }

    #[test]
    fn keeps_measured_width_for_ordinary_flex_items() {
        let mut node = Node {
            path: "article".into(),
            parent: None,
            tag: "article".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 253.0,
                height: 236.0,
            },
            style: Styles::from([
                ("width".into(), "253px".into()),
                ("height".into(), "185px".into()),
            ]),
            before: None,
            after: None,
        };
        node.attributes.insert("class".into(), "card".into());
        let captured = node.clone();
        normalize(
            &mut node.style,
            &captured,
            &[".card { flex: 0 0 auto; height: auto; transition: box-shadow .2s; }".into()],
        );
        assert_eq!(node.style["width"], "253px");
        assert_eq!(node.style["height"], "185px");
        assert_eq!(node.style["flex"], "0 0 auto");
    }

    #[test]
    fn removes_measured_width_from_growing_flex_items() {
        let mut node = Node {
            path: "article>div".into(),
            parent: Some("article".into()),
            tag: "div".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 320.0,
                height: 80.0,
            },
            style: Styles::from([("width".into(), "320px".into())]),
            before: None,
            after: None,
        };
        node.attributes.insert("class".into(), "content".into());
        let captured = node.clone();
        normalize(
            &mut node.style,
            &captured,
            &[".content { flex: 1 1 0%; min-width: 0; }".into()],
        );
        assert!(!node.style.contains_key("width"));
        assert_eq!(node.style["flex"], "1 1 0%");
        assert_eq!(node.style["min-width"], "0");
    }

    #[test]
    fn removes_sampled_height_from_intrinsic_flex_cards() {
        let mut node = Node {
            path: "article>button".into(),
            parent: Some("article".into()),
            tag: "button".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 369.0,
                height: 245.0,
            },
            style: Styles::from([
                ("display".into(), "flex".into()),
                ("height".into(), "245px".into()),
                ("overflow".into(), "hidden".into()),
            ]),
            before: None,
            after: None,
        };
        node.attributes.insert("class".into(), "task-card".into());
        let captured = node.clone();
        normalize(
            &mut node.style,
            &captured,
            &[".task-card { display: flex; min-height: 132px; overflow: hidden; }".into()],
        );
        assert!(!node.style.contains_key("height"));
        assert_eq!(node.style["min-height"], "132px");
        assert_eq!(node.style["overflow"], "hidden");
    }

    #[test]
    fn rejects_authored_layout_values_from_inactive_media_rules() {
        let mut node = Node {
            path: "header".into(),
            parent: None,
            tag: "div".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 40.0,
            },
            style: Styles::from([
                ("display".into(), "flex".into()),
                ("flex-direction".into(), "row".into()),
                ("gap".into(), "normal".into()),
            ]),
            before: None,
            after: None,
        };
        node.attributes.insert("class".into(), "header".into());
        let captured = node.clone();
        normalize(
            &mut node.style,
            &captured,
            &[".header { flex-direction: column; gap: 4px; }".into()],
        );
        assert_eq!(node.style["flex-direction"], "row");
        assert_eq!(node.style["gap"], "normal");
    }

    #[test]
    fn ignores_declarations_for_descendants() {
        let mut node = Node {
            path: "button".into(),
            parent: None,
            tag: "button".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 36.0,
            },
            style: Styles::from([
                ("width".into(), "200px".into()),
                ("height".into(), "36px".into()),
            ]),
            before: None,
            after: None,
        };
        node.attributes.insert("class".into(), "menu-row".into());
        let captured = node.clone();
        normalize(
            &mut node.style,
            &captured,
            &[
                ".menu-row { width: 100%; height: 36px; display: flex; }".into(),
                ".menu-row svg { width: 20px; height: 20px; flex-shrink: 0; }".into(),
            ],
        );
        assert_eq!(node.style["width"], "100%");
        assert_eq!(node.style["height"], "36px");
        assert!(!node.style.contains_key("flex-shrink"));
    }

    #[test]
    fn restores_centered_max_width_container() {
        let mut node = Node {
            path: "main".into(),
            parent: None,
            tag: "main".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 177.0,
                y: 0.0,
                width: 1076.0,
                height: 1000.0,
            },
            style: Styles::from([
                ("width".into(), "1076px".into()),
                ("margin-left".into(), "0px".into()),
                ("margin-right".into(), "0px".into()),
            ]),
            before: None,
            after: None,
        };
        node.attributes.insert("class".into(), "content".into());
        let captured = node.clone();
        normalize(
            &mut node.style,
            &captured,
            &[".content { max-width: 1076px; margin: 0 auto; }".into()],
        );
        assert!(!node.style.contains_key("width"));
        assert_eq!(node.style["max-width"], "1076px");
        assert_eq!(node.style["margin-left"], "auto");
        assert_eq!(node.style["margin-right"], "auto");
    }

    #[test]
    fn leaves_conflicting_responsive_declarations_captured() {
        let mut node = Node {
            path: "section".into(),
            parent: None,
            tag: "section".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 760.0,
                height: 100.0,
            },
            style: Styles::from([("grid-template-columns".into(), "372px 372px".into())]),
            before: None,
            after: None,
        };
        node.attributes.insert("class".into(), "grid".into());
        let captured = node.clone();
        normalize(
            &mut node.style,
            &captured,
            &[
                ".grid { display: grid; grid-template-columns: repeat(2, 1fr); }".into(),
                ".grid { grid-template-columns: 1fr; }".into(),
            ],
        );
        assert_eq!(node.style["display"], "grid");
        assert_eq!(node.style["grid-template-columns"], "372px 372px");
    }
}
