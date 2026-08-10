use super::{
    jsx_state_changes::*, jsx_state_existing::*, jsx_state_portals::*, jsx_state_roots::*,
    jsx_variants, tree,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn overlay(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
    known_surface_roots: Option<&std::collections::HashSet<String>>,
    render_flags: (bool, bool),
) -> String {
    let (include_changed_existing, preserve_mounted_controls) = render_flags;
    let alignment = super::sibling_alignment::of(state, baseline);
    if let Some(surface_roots) = known_surface_roots {
        let baseline_paths = baseline
            .nodes
            .iter()
            .map(|node| node.path.as_str())
            .collect::<BTreeSet<_>>();
        let existing = surface_roots
            .iter()
            .filter(|root| {
                // A path present in both captures can denote two different elements, because
                // an insertion renames every following same-tag sibling. Diffing an inserted
                // element against whatever used to hold its path reports the insertion as a
                // change to the displaced element.
                alignment.insertion(root).is_none()
            })
            .filter(|root| {
                let state_node = state.nodes.iter().find(|node| node.path == root.as_str());
                let baseline_node = baseline
                    .nodes
                    .iter()
                    .find(|node| node.path == root.as_str());
                state_node
                    .zip(baseline_node)
                    .is_some_and(|(state_node, baseline_node)| {
                        compatible_surface(state_node, baseline_node)
                            && compatible_children(state, baseline, root)
                            && (include_changed_existing
                                || preserve_mounted_controls
                                || state.nodes.iter().any(|node| {
                                    matches!(node.tag.as_str(), "input" | "textarea")
                                        && (node.path == root.as_str()
                                            || descendant_of(&node.path, root))
                                })
                                || compatible_descendants(state, baseline, root))
                    })
            })
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let structural = surface_roots
            .iter()
            .filter(|root| !existing.contains(*root))
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let replacements = structural
            .iter()
            .filter(|root| {
                baseline_paths.contains(root.as_str())
                    && alignment.insertion(root).is_none()
            })
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let mut activated = existing.clone();
        let shifted = structural
            .iter()
            .filter_map(|root| {
                let insertion = alignment.insertion(root)?;
                Some((insertion.displaced.clone()?, root.clone()))
            })
            .collect::<Vec<_>>();
        activated.retain(|path| {
            !shifted
                .iter()
                .any(|(state_root, _)| path == state_root || descendant_of(path, state_root))
        });
        if include_changed_existing {
            let mut changed = changed_existing_paths(state, baseline, &structural);
            changed.retain(|path| {
                !shifted
                    .iter()
                    .any(|(state_root, _)| path == state_root || descendant_of(path, state_root))
            });
            activated.extend(changed);
        }
        let mut delta_roots = activated.clone();
        delta_roots.extend(shifted.iter().map(|(state_root, _)| state_root.clone()));
        let activation = existing_surface(
            state,
            baseline,
            components,
            &activated,
            &existing,
            &delta_roots,
            &shifted,
            &alignment,
        );
        let added = state
            .nodes
            .iter()
            .filter(|node| {
                structural.contains(&node.path)
                    && !replacements.contains(&node.path)
                    // A survivor keeps its identity and its place in the static markup, so
                    // portalling it would render the same element twice.
                    && !shifted.iter().any(|(displaced, _)| *displaced == node.path)
            })
            .collect();
        let added_portals = inserted_surfaces(added, state, baseline, components, assets, handlers);
        let replacement_portals =
            replacement_surfaces(state, components, assets, handlers, &replacements);
        if !activation.is_empty() || !replacement_portals.is_empty() || !added_portals.is_empty() {
            return format!("<>{added_portals}{replacement_portals}{activation}</>");
        }
        return portals(
            state
                .nodes
                .iter()
                .filter(|node| surface_roots.contains(&node.path))
                .collect(),
            components,
            assets,
            handlers,
        );
    }
    let baseline_paths = baseline
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .collect::<BTreeSet<_>>();
    let surface_roots = crate::interaction_surface::roots(state, baseline);
    let roots = state
        .nodes
        .iter()
        .filter(|node| {
            surface_roots.contains(&node.path)
                || (!baseline_paths.contains(node.path.as_str())
                    && node
                        .parent
                        .as_deref()
                        .is_none_or(|parent| baseline_paths.contains(parent)))
        })
        .collect::<Vec<_>>();
    let added_count = state
        .nodes
        .iter()
        .filter(|node| !baseline_paths.contains(node.path.as_str()))
        .count();
    if roots.len() > 3 && added_count > 100 || roots.is_empty() {
        let page = jsx_variants::page(state, components, assets, handlers);
        return format!(
            "createPortal(<div className=\"recreateInteractionLayer\">{page}</div>,document.body)"
        );
    }
    portals(roots, components, assets, handlers)
}


