use super::tree;
use std::collections::BTreeSet;

pub(super) fn existing_surface(
    state: &crate::model::PageState,
    components: &tree::Components,
    roots: &std::collections::HashSet<String>,
    marked_roots: &std::collections::HashSet<String>,
    delta_roots: &std::collections::HashSet<String>,
    shifted: &[(String, String)],
    alignment: &crate::node_alignment::Alignment,
) -> String {
    if roots.is_empty() && delta_roots.is_empty() {
        return String::new();
    }
    let entries = state
        .nodes
        .iter()
        .filter(|node| {
            !shifted.iter().any(|(state_root, _)| {
                node.path == *state_root || descendant_of(&node.path, state_root)
            })
        })
        .filter(|node| {
            roots.iter().any(|root| {
                node.path == *root
                    || node
                        .path
                        .strip_prefix(root)
                        .is_some_and(|suffix| suffix.starts_with('>'))
            })
        })
        .filter(|node| {
            alignment.counterpart(&node.path).is_some_and(|baseline| {
                node.before != baseline.before || node.after != baseline.after
            })
        })
        .filter_map(|node| {
            components
                .classes
                .get(&node.path)
                .map(|class| (&node.path, class))
        })
        .collect::<Vec<_>>();
    let mut hidden = alignment
        .removals()
        .iter()
        .filter(|removal| removal.tag != "#text")
        .filter(|removal| {
            contained_by(delta_roots, &removal.parent)
                || marked_roots.iter().any(|root| {
                    removal
                        .path
                        .strip_prefix(root)
                        .is_some_and(|suffix| suffix.starts_with('>'))
                })
        })
        .map(|removal| removal.path.clone())
        .collect::<BTreeSet<_>>();
    let hidden_paths = hidden.clone();
    hidden.retain(|path| {
        !hidden_paths.iter().any(|parent| {
            parent != path
                && path
                    .strip_prefix(parent)
                    .is_some_and(|suffix| suffix.starts_with('>'))
        })
    });
    let styles = state
        .nodes
        .iter()
        .filter(|node| node.tag != "#text" && contained_by(delta_roots, &node.path))
        .filter_map(|node| {
            let baseline = alignment.counterpart(&node.path)?;
            let shifted_node = baseline.path != node.path;
            let changed = node
                .style
                .iter()
                .filter(|(name, value)| {
                    baseline.style.get(*name) != Some(*value)
                        || shifted_node && state_sensitive_property(name)
                })
                .collect::<Vec<_>>();
            (!changed.is_empty()).then_some((&node.path, changed))
        })
        .collect::<Vec<_>>();
    let texts = state
        .nodes
        .iter()
        .filter(|node| node.tag == "#text" && contained_by(delta_roots, &node.path))
        .filter_map(|node| {
            let baseline = alignment.counterpart(&node.path);
            (baseline.is_some_and(|baseline| node.text != baseline.text)
                || baseline.is_none() && contained_by(marked_roots, &node.path))
            .then_some((&node.path, &node.text))
        })
        .collect::<Vec<_>>();
    let attributes = state
        .nodes
        .iter()
        .filter(|node| node.tag != "#text" && contained_by(delta_roots, &node.path))
        .filter_map(|node| {
            let baseline = alignment.counterpart(&node.path)?;
            let names = node
                .attributes
                .keys()
                .chain(baseline.attributes.keys())
                .filter(|name| !matches!(name.as_str(), "class" | "style"))
                .collect::<BTreeSet<_>>();
            let changed = names
                .into_iter()
                .filter(|name| node.attributes.get(*name) != baseline.attributes.get(*name))
                .map(|name| (name, node.attributes.get(name)))
                .collect::<Vec<_>>();
            (!changed.is_empty()).then_some((&node.path, changed))
        })
        .collect::<Vec<_>>();
    format!(
        "<ExistingSurface entries={{{}}} roots={{{}}} hidden={{{}}} styles={{{}}} texts={{{}}} attributes={{{}}}/>",
        serde_json::to_string(&entries).expect("surface classes should serialize"),
        serde_json::to_string(marked_roots).expect("surface roots should serialize"),
        serde_json::to_string(&hidden).expect("hidden paths should serialize"),
        serde_json::to_string(&styles).expect("surface styles should serialize"),
        serde_json::to_string(&texts).expect("surface text should serialize"),
        serde_json::to_string(&attributes).expect("surface attributes should serialize")
    )
}

pub(super) fn contained_by(roots: &std::collections::HashSet<String>, path: &str) -> bool {
    roots
        .iter()
        .any(|root| path == root || descendant_of(path, root))
}

pub(super) fn descendant_of(path: &str, root: &str) -> bool {
    path.strip_prefix(root)
        .is_some_and(|suffix| suffix.starts_with('>'))
}

pub(super) fn state_sensitive_property(name: &str) -> bool {
    name == "animation"
        || name.starts_with("animation-")
        || matches!(
            name,
            "-webkit-text-fill-color"
                | "background-color"
                | "color"
                | "filter"
                | "opacity"
                | "pointer-events"
                | "transform"
                | "visibility"
                | "z-index"
        )
}
