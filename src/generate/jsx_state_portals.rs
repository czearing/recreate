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
    let alignment = crate::node_alignment::of(state, baseline);
    roots
        .into_iter()
        .filter_map(|node| {
            let parent = node.parent.as_ref()?;
            let rendered = jsx::render(&node.path, components, assets, 0, true, handlers);
            let before = alignment
                .insertion(&node.path)
                .and_then(|insertion| insertion.before.clone())
                .map(|before| serde_json::to_string(&before).expect("anchor path should serialize"))
                .unwrap_or_else(|| "null".into());
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
