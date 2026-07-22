use crate::model::PageState;

pub fn append_responsive(states: &[PageState], css: &mut String) {
    let Some(base) = states.first() else {
        return;
    };
    let mut base_rules = String::new();
    append(base, &mut base_rules);
    css.push_str(&base_rules);
    let mut responsive: Vec<_> = states.iter().skip(1).collect();
    responsive.sort_by_key(|state| std::cmp::Reverse(state.viewport.width));
    for (index, state) in responsive.iter().enumerate() {
        let mut rules = String::new();
        append(state, &mut rules);
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

pub fn append(state: &PageState, css: &mut String) {
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
    append_scoped_custom_properties(state, css);
    for property in [
        "-webkit-box-orient",
        "-webkit-font-smoothing",
        "caret-color",
        "color",
        "isolation",
        "justify-self",
        "overflow-wrap",
        "resize",
        "text-overflow",
        "user-select",
        "visibility",
        "-webkit-line-clamp",
        "z-index",
    ] {
        append_inherited_fallback(state, property, css);
    }
    append_forced_properties(state, css);
}

fn append_scoped_custom_properties(state: &PageState, css: &mut String) {
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
            css.push_str(&format!("{path}{{{declarations}}}\n"));
        }
    }
}

fn append_forced_properties(state: &PageState, css: &mut String) {
    let Some(root) = state.dom.get("html") else {
        return;
    };
    for (property, tags) in [
        (
            "z-index",
            &["a", "button", "input", "select", "textarea"][..],
        ),
        ("resize", &["textarea"][..]),
        ("content-visibility", &["i"][..]),
    ] {
        let Some(index) = root
            .computed_style_properties
            .iter()
            .position(|candidate| candidate == property)
        else {
            continue;
        };
        for node in state
            .nodes
            .iter()
            .filter(|node| tags.contains(&node.tag.as_str()))
        {
            let Some(dom) = state.dom.get(&node.path) else {
                continue;
            };
            if let Some(value) = style_value(root, dom, index) {
                css.push_str(&format!("{}{{{property}:{value};}}\n", node.path));
            }
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
            Some(format!("{property}:{value};"))
        })
        .collect()
}

fn append_inherited_fallback(state: &PageState, property: &str, css: &mut String) {
    let Some(root) = state.dom.get("html") else {
        return;
    };
    let Some(index) = root
        .computed_style_properties
        .iter()
        .position(|candidate| candidate == property)
    else {
        return;
    };
    for (path, node) in &state.dom {
        if node.node_type != 1 || path.contains("#text") {
            continue;
        }
        let value = style_value(root, node, index);
        let parent = node
            .physical_parent
            .as_deref()
            .and_then(|path| state.dom.get(path))
            .and_then(|parent| style_value(root, parent, index));
        if let Some(value) = value
            && Some(value) != parent
        {
            css.push_str(&format!("{path}{{{property}:{value};}}\n"));
        }
    }
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
}
