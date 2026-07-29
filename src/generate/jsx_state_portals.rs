use super::{jsx, tree};
use std::collections::BTreeMap;

pub(super) fn replacement_surfaces(
    state: &crate::model::PageState,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
    roots: &std::collections::HashSet<String>,
) -> String {
    state
        .nodes
        .iter()
        .filter(|node| roots.contains(&node.path))
        .map(|node| {
            let rendered = jsx::render_children(&node.path, components, assets, 0, handlers);
            let class = components
                .classes
                .get(&node.path)
                .expect("replacement surface should have a class");
            format!(
                "<ReplacementSurface path={{{}}} className={{{}}}>{rendered}</ReplacementSurface>",
                serde_json::to_string(&node.path).expect("surface path should serialize"),
                serde_json::to_string(class).expect("surface class should serialize")
            )
        })
        .collect()
}

pub(super) fn inserted_surfaces(
    roots: Vec<&crate::model::Node>,
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
) -> String {
    roots
        .into_iter()
        .filter_map(|node| {
            let parent = node.parent.as_ref()?;
            let rendered = jsx::render(&node.path, components, assets, 0, true, handlers);
            let before = if shifted_insertion(&node.path, state, baseline) {
                serde_json::to_string(&node.path).unwrap()
            } else {
                "null".into()
            };
            let floating = node
                .style
                .get("position")
                .is_some_and(|position| matches!(position.as_str(), "absolute" | "fixed"))
                || node.attributes.contains_key("data-portal-node");
            Some(format!(
                "<InsertedSurface parentPath={{{}}} beforePath={{{before}}} floating={{{floating}}}>{rendered}</InsertedSurface>",
                serde_json::to_string(parent).expect("surface parent path should serialize"),
            ))
        })
        .collect()
}

pub(super) fn shifted_insertion(
    path: &str,
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
) -> bool {
    let Some(next) = next_sibling_path(path) else {
        return false;
    };
    let baseline_node = baseline.nodes.iter().find(|node| node.path == path);
    let shifted = state.nodes.iter().find(|node| node.path == next);
    baseline_node
        .zip(shifted)
        .is_some_and(|(baseline_node, shifted)| {
            baseline_node.tag == shifted.tag
                && baseline_node.attributes == shifted.attributes
                && child_signature(baseline, path) == child_signature(state, &next)
        })
}

pub(super) fn next_sibling_path(path: &str) -> Option<String> {
    let (prefix, suffix) = path.rsplit_once(":nth-of-type(")?;
    let index = suffix.strip_suffix(')')?.parse::<usize>().ok()?;
    Some(format!("{prefix}:nth-of-type({})", index + 1))
}

pub(super) fn child_signature(
    state: &crate::model::PageState,
    path: &str,
) -> Vec<(String, Option<String>)> {
    state
        .nodes
        .iter()
        .filter(|node| node.parent.as_deref() == Some(path))
        .map(|node| (node.tag.clone(), node.attributes.get("role").cloned()))
        .collect()
}

pub(super) fn portals(
    roots: Vec<&crate::model::Node>,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
) -> String {
    let portals = roots
        .into_iter()
        .map(|node| {
            let content = jsx::render(&node.path, components, assets, 2, true, handlers);
            let parent = serde_json::to_string(node.parent.as_deref().unwrap_or("body"))
                .expect("DOM path should serialize");
            format!(
                "{{createPortal(<>{content}</>,document.querySelector({parent})||document.body)}}"
            )
        })
        .collect::<String>();
    format!("<>{portals}</>")
}

pub(super) fn trigger_portals(
    roots: Vec<&crate::model::Node>,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
    trigger: usize,
) -> String {
    let portals = roots
        .into_iter()
        .map(|node| {
            let content = jsx::render(&node.path, components, assets, 2, true, handlers);
            format!("<AnchoredSurface trigger={{{trigger}}}>{content}</AnchoredSurface>")
        })
        .collect::<String>();
    format!("<>{portals}</>")
}
