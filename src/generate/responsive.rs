use super::css::declarations;
use crate::model::{Node, Pseudo, Specification, Styles, Viewport};
use std::collections::{BTreeMap, HashMap, HashSet};

pub fn append_filtered(
    specification: &Specification,
    assets: &BTreeMap<String, String>,
    classes: &BTreeMap<String, String>,
    css: &mut String,
    paths: Option<&HashSet<String>>,
    fluid_heights: &HashSet<String>,
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
        let shrunk_roots = state
            .nodes
            .iter()
            .filter(|node| {
                base_nodes.get(node.path.as_str()).is_some_and(|base| {
                    shrunk_flex_item(
                        base,
                        node,
                        node.parent
                            .as_deref()
                            .and_then(|parent| state_nodes.get(parent).copied()),
                    )
                })
            })
            .map(|node| node.path.as_str())
            .collect::<HashSet<_>>();
        for node in &state.nodes {
            if paths.is_some_and(|paths| !paths.contains(&node.path)) {
                continue;
            }

            let (Some(base_node), Some(class)) =
                (base_nodes.get(node.path.as_str()), classes.get(&node.path))
            else {
                continue;
            };
            rules.push_str(&append_node_rules(
                base_node,
                node,
                node.parent
                    .as_deref()
                    .and_then(|parent| state_nodes.get(parent).copied()),
                (&base.viewport, &state.viewport),
                class,
                assets,
                &state.css_rules,
                fluid_heights.contains(&node.path),
                constrained_by_flex_chain(node, &shrunk_roots, &state_nodes),
            ));
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

fn constrained_by_flex_chain(
    node: &Node,
    roots: &HashSet<&str>,
    nodes: &HashMap<&str, &Node>,
) -> bool {
    let mut parent = node.parent.as_deref();
    while let Some(path) = parent {
        if roots.contains(path) {
            return true;
        }
        let Some(node) = nodes.get(path) else {
            return false;
        };
        if node.style.get("display").map(String::as_str) != Some("flex")
            || node.style.get("flex-direction").map(String::as_str) != Some("row")
        {
            return false;
        }
        parent = node.parent.as_deref();
    }
    false
}

pub(super) fn band(
    width: u32,
    smaller: Option<u32>,
    wider: u32,
    sparse: bool,
) -> (Option<u32>, u32) {
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
    css_rules: &[String],
    fluid_height: bool,
    text_parent: bool,
) -> String {
    let mut styles = node.style.clone();
    let authored_width = super::authored_css::has_property(node, css_rules, "width");
    super::authored_css::normalize(&mut styles, node, css_rules);
    if !authored_width
        && (intrinsic_flex_text(node, parent, text_parent) || fluid_flex_item(node, parent))
    {
        styles.remove("width");
    }
    if fluid_height {
        styles.remove("height");
    }
    super::inherited_styles::normalize(&mut styles, node, parent, css_rules);
    super::responsive_geometry::normalize(&mut styles, node, parent, viewport, None);
    declarations(&styles, assets)
}

fn intrinsic_flex_text(node: &Node, parent: Option<&Node>, text_parent: bool) -> bool {
    let Some(parent) = parent else {
        return false;
    };
    text_parent
        && matches!(node.tag.as_str(), "div" | "p" | "span")
        && node.style.get("display").map(String::as_str) == Some("block")
        && node.style.get("position").map(String::as_str) == Some("static")
        && parent.style.get("display").map(String::as_str) == Some("flex")
        && parent.style.get("flex-direction").map(String::as_str) == Some("column")
        && node.rect.width < parent.rect.width - 12.0
}

fn fluid_flex_item(node: &Node, parent: Option<&Node>) -> bool {
    parent.is_some_and(|parent| {
        let flexible_main_axis = ["flex-grow", "flex-shrink"].into_iter().any(|name| {
            node.style
                .get(name)
                .and_then(|value| value.parse::<f64>().ok())
                .is_some_and(|value| value > 0.0)
        });
        parent.style.get("display").map(String::as_str) == Some("flex")
            && (parent
                .style
                .get("flex-direction")
                .map(String::as_str)
                .is_none_or(|direction| direction.starts_with("row"))
                && flexible_main_axis
                || (parent
                    .style
                    .get("align-items")
                    .map(String::as_str)
                    .is_none_or(|alignment| matches!(alignment, "normal" | "stretch"))
                    && node
                        .style
                        .get("align-self")
                        .map(String::as_str)
                        .is_none_or(|alignment| {
                            matches!(alignment, "auto" | "normal" | "stretch")
                        })))
            && !matches!(
                node.style.get("position").map(String::as_str),
                Some("absolute" | "fixed")
            )
            && node.attributes.get("role").is_none_or(|role| role != "img")
            && !(node.rect.width <= 32.0 && node.rect.height <= 32.0)
            && !matches!(
                node.tag.as_str(),
                "button" | "canvas" | "img" | "input" | "select" | "svg" | "textarea" | "video"
            )
    })
}

#[allow(clippy::too_many_arguments)]
fn append_node_rules(
    base: &Node,
    node: &Node,
    parent: Option<&Node>,
    viewports: (&Viewport, &Viewport),
    class: &str,
    assets: &BTreeMap<String, String>,
    css_rules: &[String],
    fluid_height: bool,
    constrained_by_flex: bool,
) -> String {
    let mut rules = String::new();
    let (base_viewport, viewport) = viewports;
    let mut changed = changed_styles(&base.style, &node.style);
    super::authored_css::normalize(&mut changed, node, css_rules);
    if constrained_by_flex
        && node
            .style
            .get("flex-shrink")
            .and_then(|value| value.parse::<f64>().ok())
            .is_some_and(|value| value > 0.0)
        && node.rect.width + 1.0 < base.rect.width
    {
        changed.remove("width");
        changed.remove("inline-size");
    }
    if !super::authored_css::has_property(node, css_rules, "width") && fluid_flex_item(node, parent)
    {
        if shrunk_flex_item(base, node, parent) {
            changed.insert("width".into(), "100%".into());
            changed.insert("max-width".into(), format!("{}px", node.rect.width));
        } else {
            changed.remove("width");
        }
    }
    if fluid_height {
        changed.remove("height");
    }
    super::inherited_styles::normalize(&mut changed, node, parent, css_rules);
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
        &mut rules,
    );
    append_pseudo_rule(
        class,
        "after",
        base.after.as_ref(),
        node.after.as_ref(),
        assets,
        &mut rules,
    );
    rules
}

fn shrunk_flex_item(base: &Node, node: &Node, parent: Option<&Node>) -> bool {
    parent.is_some_and(|parent| {
        parent.style.get("display").map(String::as_str) == Some("flex")
            && parent.style.get("flex-direction").map(String::as_str) == Some("row")
            && node
                .style
                .get("flex-shrink")
                .and_then(|value| value.parse::<f64>().ok())
                .is_some_and(|value| value > 0.0)
            && node.rect.width + 1.0 < base.rect.width
    })
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

pub(super) fn media_rule(minimum: Option<u32>, maximum: u32, rules: &str) -> String {
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
    parent: Option<&Node>,
    viewport: &Viewport,
    base: Option<(&Node, &Viewport)>,
) {
    super::responsive_geometry::normalize(styles, node, parent, viewport, base);
}

#[cfg(test)]
#[path = "responsive_tests.rs"]
mod tests;
