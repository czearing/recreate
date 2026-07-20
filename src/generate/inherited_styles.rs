use crate::model::{Node, Styles};

const PROPERTIES: &[&str] = &["-webkit-text-fill-color", "color", "fill", "stroke"];

pub fn normalize(styles: &mut Styles, node: &Node, parent: Option<&Node>, rules: &[String]) {
    let Some(parent) = parent else {
        return;
    };
    for property in PROPERTIES {
        if disabled_control(node) && matches!(*property, "color" | "-webkit-text-fill-color") {
            continue;
        }
        if let Some(value) = authored_value(node, rules, property) {
            styles.insert((*property).into(), value);
        } else if styles.get(*property) == parent.style.get(*property)
            && !(matches!(
                node.tag.as_str(),
                "button" | "input" | "select" | "textarea"
            ) && matches!(*property, "color" | "-webkit-text-fill-color"))
        {
            styles.remove(*property);
        }
    }

    fn disabled_control(node: &Node) -> bool {
        matches!(
            node.tag.as_str(),
            "button" | "input" | "select" | "textarea"
        ) && (node.attributes.contains_key("disabled")
            || node
                .attributes
                .get("aria-disabled")
                .is_some_and(|value| value == "true"))
    }
}

fn authored_value(node: &Node, rules: &[String], property: &str) -> Option<String> {
    let classes: Vec<_> = node
        .attributes
        .get("class")
        .into_iter()
        .flat_map(|value| value.split_whitespace())
        .collect();
    let values: Vec<_> = rules
        .iter()
        .filter_map(|rule| rule.split_once('{'))
        .filter(|(selector, _)| {
            !selector.starts_with('@')
                && !selector.contains(':')
                && classes
                    .iter()
                    .any(|class| directly_targets(selector, class))
        })
        .flat_map(|(_, declarations)| declarations.split(';'))
        .filter_map(|declaration| declaration.split_once(':'))
        .filter(|(name, value)| name.trim() == property && !value.contains("var("))
        .map(|(_, value)| value.trim().to_string())
        .collect();
    let first = values.first()?;
    values
        .iter()
        .all(|value| value == first)
        .then(|| first.clone())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Rect, Styles};

    fn node(tag: &str, class: &str, styles: &[(&str, &str)]) -> Node {
        let mut node = Node {
            path: tag.into(),
            parent: None,
            tag: tag.into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 16.0,
                height: 16.0,
            },
            style: Styles::from_iter(
                styles
                    .iter()
                    .map(|(name, value)| ((*name).into(), (*value).into())),
            ),
            before: None,
            after: None,
        };
        node.attributes.insert("class".into(), class.into());
        node
    }

    #[test]
    fn removes_computed_inherited_paint() {
        let parent = node("span", "control", &[("color", "rgb(138, 136, 134)")]);
        let child = node("path", "", &[("color", "rgb(138, 136, 134)")]);
        let mut styles = child.style.clone();
        normalize(&mut styles, &child, Some(&parent), &[]);
        assert!(!styles.contains_key("color"));
    }

    #[test]
    fn preserves_authored_current_color() {
        let parent = node("span", "control", &[("fill", "rgb(0, 0, 0)")]);
        let child = node("svg", "icon", &[("fill", "rgb(138, 136, 134)")]);
        let mut styles = child.style.clone();
        normalize(
            &mut styles,
            &child,
            Some(&parent),
            &[".icon{fill:currentColor;}".into()],
        );
        assert_eq!(styles["fill"], "currentColor");
    }

    #[test]
    fn preserves_form_control_text_color() {
        let parent = node("div", "", &[("color", "rgb(36, 36, 36)")]);
        let child = node("button", "", &[("color", "rgb(36, 36, 36)")]);
        let mut styles = child.style.clone();
        normalize(&mut styles, &child, Some(&parent), &[]);
        assert_eq!(styles["color"], "rgb(36, 36, 36)");
    }

    #[test]
    fn preserves_computed_disabled_control_paint() {
        let parent = node("div", "", &[("color", "rgb(36, 36, 36)")]);
        let mut child = node(
            "button",
            "send",
            &[
                ("color", "rgb(176, 176, 176)"),
                ("-webkit-text-fill-color", "rgb(176, 176, 176)"),
            ],
        );
        child.attributes.insert("disabled".into(), String::new());
        let mut styles = child.style.clone();
        normalize(
            &mut styles,
            &child,
            Some(&parent),
            &[".send{color:white;-webkit-text-fill-color:white;}".into()],
        );
        assert_eq!(styles["color"], "rgb(176, 176, 176)");
        assert_eq!(styles["-webkit-text-fill-color"], "rgb(176, 176, 176)");
    }
}
