use crate::model::PageState;
use std::collections::{BTreeMap, BTreeSet};

/// The names this layer owns: every custom property some captured state gives a
/// value to. It states each one per viewport condition, so any producer that
/// writes an unguarded fallback must leave these alone -- restating one both
/// duplicates the rule and reasserts, below a breakpoint, a value the source may
/// have declared only above it.
pub fn declared_names(states: &[PageState]) -> BTreeSet<String> {
    states
        .iter()
        .filter_map(|state| state.dom.get("html"))
        .flat_map(|root| {
            root.computed_style_properties
                .iter()
                .zip(&root.computed_style_values)
                .filter(|(property, _)| property.starts_with("--"))
                .filter(|(_, value)| {
                    root.computed_style_dictionary
                        .get(**value as usize)
                        .is_some_and(|value| !value.trim().is_empty())
                })
                .map(|(property, _)| property.clone())
        })
        .collect()
}

pub fn append_responsive(
    states: &[PageState],
    classes: &BTreeMap<String, String>,
    css: &mut String,
) {
    let Some(base) = states.first() else {
        return;
    };
    let mut base_rules = String::new();
    append(base, classes, &mut base_rules);
    css.push_str(&base_rules);
    let mut responsive: Vec<_> = states.iter().skip(1).collect();
    responsive.sort_by_key(|state| std::cmp::Reverse(state.viewport.width));
    for (index, state) in responsive.iter().enumerate() {
        let mut rules = String::new();
        append(state, classes, &mut rules);
        let rules = super::custom_property_diff::against(&base_rules, &rules);
        let wider = if index == 0 {
            base.viewport.width
        } else {
            responsive[index - 1].viewport.width
        };
        let smaller = responsive.get(index + 1).map(|next| next.viewport.width);
        let (minimum, maximum) =
            super::responsive::band(state.viewport.width, smaller, wider, responsive.len() == 1);
        css.push_str(&super::responsive::media_rule(minimum, maximum, &rules));
    }
}

pub fn append(state: &PageState, classes: &BTreeMap<String, String>, css: &mut String) {
    let Some(root) = state.dom.get("html") else {
        return;
    };
    let declarations = render(
        &root.computed_style_properties,
        &root.computed_style_dictionary,
        &root.computed_style_values,
    );
    if !declarations.is_empty() {
        css.push_str(":root{");
        css.push_str(&declarations);
        css.push_str("}\n");
    }
    append_scoped_custom_properties(state, classes, css);
}

fn append_scoped_custom_properties(
    state: &PageState,
    classes: &BTreeMap<String, String>,
    css: &mut String,
) {
    let Some(root) = state.dom.get("html") else {
        return;
    };
    for (path, node) in &state.dom {
        if path == "html" || node.node_type != 1 || path.contains("#text") {
            continue;
        }
        let parent = node
            .physical_parent
            .as_deref()
            .and_then(|path| state.dom.get(path));
        let declarations = root
            .computed_style_properties
            .iter()
            .enumerate()
            .filter(|(_, property)| property.starts_with("--"))
            .filter_map(|(index, property)| {
                let value = style_value(root, node, index)?;
                let parent = parent.and_then(|parent| style_value(root, parent, index));
                (!value.is_empty() && Some(value) != parent).then(|| format!("{property}:{value};"))
            })
            .collect::<String>();
        if !declarations.is_empty() {
            let Some(selector) = class_selector(classes, path) else {
                continue;
            };
            css.push_str(&format!("{selector}{{{declarations}}}\n"));
        }
    }
}

fn render(properties: &[String], dictionary: &[String], values: &[u32]) -> String {
    properties
        .iter()
        .zip(values)
        .filter(|(property, _)| property.starts_with("--"))
        .filter_map(|(property, value)| {
            let value = dictionary.get(*value as usize)?;
            // An empty custom property is not a value. Emitting it makes every
            // `var()` that reads it invalid at computed-value time, so the property
            // silently falls back to its initial value instead of the authored one.
            (!value.trim().is_empty()).then(|| format!("{property}:{value};"))
        })
        .collect()
}

fn class_selector(classes: &BTreeMap<String, String>, path: &str) -> Option<String> {
    classes.get(path).map(|class_names| {
        class_names
            .split_whitespace()
            .map(|class_name| format!(".{class_name}"))
            .collect()
    })
}

fn style_value<'a>(
    root: &'a crate::model::DomNode,
    node: &crate::model::DomNode,
    index: usize,
) -> Option<&'a str> {
    node.computed_style_values
        .get(index)
        .and_then(|value| root.computed_style_dictionary.get(*value as usize))
        .map(String::as_str)
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_complete_captured_custom_properties() {
        let properties = vec!["--brand".into(), "--spacing".into(), "color".into()];
        let dictionary = vec!["#6264a7".into(), "8px".into(), "red".into()];
        let values = vec![0, 1, 2];

        assert_eq!(
            super::render(&properties, &dictionary, &values),
            "--brand:#6264a7;--spacing:8px;"
        );
    }

    /// A name is owned by this layer only where it has a value to state. One
    /// that is empty in every state is never emitted here, so claiming it would
    /// silence the authored fallback and leave the property undeclared.
    #[test]
    fn names_only_the_custom_properties_some_state_declares() {
        let mut narrow = state(&["--changed", "--blank", "color"], &["12px", "", "red"]);
        let wide = state(
            &["--changed", "--dropped", "color"],
            &["24px", "37px", "red"],
        );
        narrow.viewport.width = 320;

        let names = super::declared_names(&[wide, narrow]);

        assert!(names.contains("--changed"));
        assert!(names.contains("--dropped"));
        assert!(!names.contains("--blank"), "{names:?}");
        assert!(!names.contains("color"), "{names:?}");
    }

    fn state(properties: &[&str], values: &[&str]) -> crate::model::PageState {
        let mut state = crate::model::PageState::default();
        let mut node = crate::model::DomNode {
            node_type: 1,
            ..Default::default()
        };
        node.computed_style_properties = properties.iter().map(|name| (*name).into()).collect();
        node.computed_style_dictionary = values.iter().map(|value| (*value).into()).collect();
        node.computed_style_values = (0..values.len() as u32).collect();
        state.dom.insert("html".into(), node);
        state
    }
}
