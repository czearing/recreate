use super::css::declarations;
use crate::model::{Node, Pseudo, Specification, Styles, Viewport};
use std::collections::{BTreeMap, HashMap};

pub fn append(
    specification: &Specification,
    assets: &BTreeMap<String, String>,
    classes: &BTreeMap<String, String>,
    css: &mut String,
) {
    let Some(base) = specification.states.first() else {
        return;
    };
    let base_nodes: HashMap<_, _> = base
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    let mut states: Vec<_> = specification.states.iter().skip(1).collect();
    states.sort_by_key(|state| std::cmp::Reverse(state.viewport.width));

    for (index, state) in states.iter().enumerate() {
        let mut rules = String::new();
        let state_nodes: HashMap<_, _> = state
            .nodes
            .iter()
            .map(|node| (node.path.as_str(), node))
            .collect();
        for node in &state.nodes {
            let (Some(base_node), Some(class)) =
                (base_nodes.get(node.path.as_str()), classes.get(&node.path))
            else {
                continue;
            };
            append_node_rules(
                base_node,
                node,
                node.parent
                    .as_deref()
                    .and_then(|parent| state_nodes.get(parent).copied()),
                (&base.viewport, &state.viewport),
                class,
                assets,
                &mut rules,
            );
        }
        if !rules.is_empty() {
            let wider = if index == 0 {
                base.viewport.width
            } else {
                states[index - 1].viewport.width
            };
            let smaller = states.get(index + 1).map(|next| next.viewport.width);
            let (minimum, maximum) = band(state.viewport.width, smaller, wider, states.len() == 1);
            css.push_str(&media_rule(minimum, maximum, &rules));
        }
    }
}

fn band(width: u32, smaller: Option<u32>, wider: u32, sparse: bool) -> (Option<u32>, u32) {
    if sparse {
        return (None, wider.saturating_sub(1).max(width));
    }
    (smaller.map(|value| value.saturating_add(1)), width)
}

pub fn base_declarations(
    node: &Node,
    parent: Option<&Node>,
    viewport: &Viewport,
    assets: &BTreeMap<String, String>,
) -> String {
    let mut styles = node.style.clone();
    super::responsive_geometry::normalize(&mut styles, node, parent, viewport, None);
    declarations(&styles, assets)
}

fn append_node_rules(
    base: &Node,
    node: &Node,
    parent: Option<&Node>,
    viewports: (&Viewport, &Viewport),
    class: &str,
    assets: &BTreeMap<String, String>,
    rules: &mut String,
) {
    let (base_viewport, viewport) = viewports;
    let mut changed = changed_styles(&base.style, &node.style);
    super::responsive_geometry::normalize(
        &mut changed,
        node,
        parent,
        viewport,
        Some((base, base_viewport)),
    );
    if !changed.is_empty() {
        rules.push_str(&format!(".{class}{{{}}}", declarations(&changed, assets)));
    }

    append_pseudo_rule(
        class,
        "before",
        base.before.as_ref(),
        node.before.as_ref(),
        assets,
        rules,
    );
    append_pseudo_rule(
        class,
        "after",
        base.after.as_ref(),
        node.after.as_ref(),
        assets,
        rules,
    );
}

fn append_pseudo_rule(
    class: &str,
    name: &str,
    base: Option<&Pseudo>,
    current: Option<&Pseudo>,
    assets: &BTreeMap<String, String>,
    rules: &mut String,
) {
    let Some(current) = current else {
        if base.is_some() {
            rules.push_str(&format!(".{class}::{name}{{content:none;}}"));
        }
        return;
    };
    let base_styles = base
        .map(|pseudo| &pseudo.style)
        .cloned()
        .unwrap_or_default();
    let changed = changed_styles(&base_styles, &current.style);
    let content = if base.is_none_or(|pseudo| pseudo.content != current.content) {
        format!("content:{};", current.content)
    } else {
        String::new()
    };
    if !content.is_empty() || !changed.is_empty() {
        rules.push_str(&format!(
            ".{class}::{name}{{{content}{}}}",
            declarations(&changed, assets)
        ));
    }
}

fn changed_styles(base: &Styles, current: &Styles) -> Styles {
    current
        .iter()
        .filter(|(key, value)| base.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn media_rule(minimum: Option<u32>, maximum: u32, rules: &str) -> String {
    match minimum {
        Some(minimum) => {
            format!("@media(min-width:{minimum}px) and (max-width:{maximum}px){{{rules}}}\n")
        }
        None => format!("@media(max-width:{maximum}px){{{rules}}}\n"),
    }
}

#[cfg(test)]
fn normalize_viewport_width(
    styles: &mut Styles,
    node: &Node,
    viewport: &Viewport,
    base: Option<(&Node, &Viewport)>,
) {
    super::responsive_geometry::normalize(styles, node, None, viewport, base);
}

#[cfg(test)]
#[path = "responsive_tests.rs"]
mod tests;
