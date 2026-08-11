pub(super) fn newly_visible_roots(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
) -> std::collections::HashSet<String> {
    let alignment = crate::node_alignment::of(state, baseline);
    let nodes = state
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();
    let mut roots = state
        .nodes
        .iter()
        .filter(|node| {
            let baseline_node = alignment.counterpart(&node.path);
            (visible(node) && baseline_node.is_none())
                || (visible_surface(node)
                    && baseline_node.is_none_or(|baseline_node| {
                        !visible(baseline_node)
                            || node.tag != baseline_node.tag
                            || node.attributes.get("role") != baseline_node.attributes.get("role")
                            || node.attributes.get("aria-modal")
                                != baseline_node.attributes.get("aria-modal")
                            || !compatible_children(state, baseline, &node.path)
                    }))
        })
        .filter_map(|node| {
            let mut root = node;
            while let Some(parent) = root
                .parent
                .as_deref()
                .and_then(|path| nodes.get(path).copied())
            {
                if parent
                    .style
                    .get("position")
                    .is_some_and(|value| matches!(value.as_str(), "absolute" | "fixed"))
                {
                    root = parent;
                    break;
                }
                if alignment.counterpart(&parent.path).is_some_and(visible) {
                    break;
                }
                root = parent;
            }
            (root.path != "html" && root.path != "html>body").then(|| root.path.clone())
        })
        .collect::<std::collections::HashSet<_>>();
    let all = roots.clone();
    roots.retain(|root| {
        !all.iter().any(|candidate| {
            candidate != root
                && root
                    .strip_prefix(candidate)
                    .is_some_and(|suffix| suffix.starts_with('>'))
        })
    });
    if roots.iter().any(|root| {
        nodes.get(root.as_str()).is_some_and(|node| {
            node.style
                .get("position")
                .is_some_and(|position| position == "fixed")
        })
    }) {
        roots.retain(|root| {
            nodes.get(root.as_str()).is_some_and(|node| {
                node.style
                    .get("position")
                    .is_some_and(|position| position == "fixed")
            })
        });
    }
    roots
}

pub(super) fn changed_structure_roots(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
) -> std::collections::HashSet<String> {
    let baseline_nodes = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();
    let state_nodes = state
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();
    let mut roots = state
        .nodes
        .iter()
        .filter(|node| {
            !matches!(node.tag.as_str(), "html" | "body")
                && node
                    .parent
                    .as_deref()
                    .and_then(|path| state_nodes.get(path))
                    .is_none_or(|parent| parent.tag != "body")
                && baseline_nodes
                    .get(node.path.as_str())
                    .is_some_and(|_| !compatible_children(state, baseline, &node.path))
        })
        .map(|node| node.path.clone())
        .collect::<std::collections::HashSet<_>>();
    let all = roots.clone();
    roots.retain(|root| {
        !all.iter().any(|candidate| {
            candidate != root
                && root
                    .strip_prefix(candidate)
                    .is_some_and(|suffix| suffix.starts_with('>'))
        })
    });
    roots
}

pub(super) fn visible_surface(node: &crate::model::Node) -> bool {
    visible(node)
        && (node
            .attributes
            .get("role")
            .is_some_and(|role| matches!(role.as_str(), "dialog" | "listbox" | "menu"))
            || node
                .style
                .get("position")
                .is_some_and(|value| matches!(value.as_str(), "absolute" | "fixed")))
}

pub(super) fn compatible_surface(
    state: &crate::model::Node,
    baseline: &crate::model::Node,
) -> bool {
    state.tag == baseline.tag && state.style.get("position") == baseline.style.get("position")
}

pub(super) fn compatible_children(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
    path: &str,
) -> bool {
    let signature = |nodes: &[crate::model::Node]| {
        nodes
            .iter()
            .filter(|node| node.parent.as_deref() == Some(path))
            .map(|node| (node.tag.clone(), node.attributes.get("role").cloned()))
            .collect::<Vec<_>>()
    };
    signature(&state.nodes) == signature(&baseline.nodes)
}

pub(super) fn compatible_descendants(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
    path: &str,
) -> bool {
    let signature = |nodes: &[crate::model::Node]| {
        nodes
            .iter()
            .filter_map(|node| {
                node.path
                    .strip_prefix(path)
                    .filter(|suffix| suffix.starts_with('>'))
                    .map(|suffix| {
                        (
                            suffix.to_string(),
                            node.tag.clone(),
                            node.attributes.get("role").cloned(),
                        )
                    })
            })
            .collect::<Vec<_>>()
    };
    signature(&state.nodes) == signature(&baseline.nodes)
}

pub(super) fn visible(node: &crate::model::Node) -> bool {
    node.rect.width > 0.0
        && node.rect.height > 0.0
        && node
            .style
            .get("display")
            .is_none_or(|value| value != "none")
        && node
            .style
            .get("visibility")
            .is_none_or(|value| value != "hidden")
        && node
            .style
            .get("opacity")
            .and_then(|value| value.parse::<f64>().ok())
            .is_none_or(|value| value > 0.01)
}
