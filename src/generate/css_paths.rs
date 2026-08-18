use crate::model::{PageState, Specification};
use std::collections::{HashMap, HashSet};

pub fn changed(specification: &Specification, baselines: &[PageState]) -> HashSet<String> {
    specification
        .states
        .iter()
        .flat_map(|state| {
            let baseline = baselines
                .iter()
                .find(|baseline| baseline.viewport.width == state.viewport.width);
            let nodes: HashMap<_, _> = baseline
                .into_iter()
                .flat_map(|baseline| &baseline.nodes)
                .map(|node| (node.path.as_str(), node))
                .collect();
            let contextual = baseline
                .map(|baseline| contextual_widths(state, baseline))
                .unwrap_or_default();
            state
                .nodes
                .iter()
                .filter(move |node| {
                    contextual.contains(&node.path)
                        || nodes.get(node.path.as_str()).is_none_or(|baseline| {
                            node.style != baseline.style || node.pseudos != baseline.pseudos
                        })
                })
                .map(|node| node.path.clone())
        })
        .collect()
}

pub fn topology(state: &PageState, baseline: &PageState) -> HashSet<String> {
    fn children(state: &PageState) -> HashMap<&str, HashSet<&str>> {
        let mut children = HashMap::<_, HashSet<_>>::new();
        for node in &state.nodes {
            if let Some(parent) = node.parent.as_deref() {
                children
                    .entry(parent)
                    .or_default()
                    .insert(node.path.as_str());
            }
        }
        children
    }
    let current = children(state);
    let captured = children(baseline);
    let baseline_nodes = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<HashMap<_, _>>();
    let changed_parents = state
        .nodes
        .iter()
        .filter_map(|node| {
            let parent = node.parent.as_deref()?;
            let changed = baseline_nodes
                .get(node.path.as_str())
                .is_none_or(|baseline| {
                    node.rect.width != baseline.rect.width
                        || node.rect.height != baseline.rect.height
                        || node.style != baseline.style
                });
            (changed || current.get(parent) != captured.get(parent)).then_some(parent)
        })
        .collect::<HashSet<_>>();
    state
        .nodes
        .iter()
        .filter(|node| {
            node.parent
                .as_deref()
                .is_some_and(|parent| changed_parents.contains(parent))
        })
        .map(|node| node.path.clone())
        .collect()
}

pub fn contextual_widths(state: &PageState, baseline: &PageState) -> HashSet<String> {
    let baseline_nodes = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<HashMap<_, _>>();
    topology(state, baseline)
        .into_iter()
        .filter(|path| {
            let Some(node) = state.nodes.iter().find(|node| node.path == *path) else {
                return false;
            };
            baseline_nodes.get(path.as_str()).is_some_and(|original| {
                let mut generated = original.style.clone();
                super::authored_css::normalize(&mut generated, original, baseline);
                let parent = original
                    .parent
                    .as_deref()
                    .and_then(|parent| baseline_nodes.get(parent).copied());
                super::inherited_styles::normalize(&mut generated, original, parent, baseline);
                super::responsive_geometry::normalize(
                    &mut generated,
                    original,
                    parent,
                    &baseline.viewport,
                    None,
                );
                let relative_width = generated
                    .get("width")
                    .is_some_and(|width| width.ends_with('%'));
                node.rect.width == original.rect.width
                    && node.rect.height == original.rect.height
                    && node.style == original.style
                    && node.rect.x != original.rect.x
                    && relative_width
            })
        })
        .collect()
}
