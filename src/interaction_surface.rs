use crate::model::{Node, Specification};
use std::collections::{HashMap, HashSet};

pub fn normalize(specification: &mut Specification) {
    for interaction in &mut specification.interactions {
        if !interaction
            .trigger_label
            .eq_ignore_ascii_case("More options")
        {
            continue;
        }
        for state in &mut interaction.states {
            let Some(baseline) = specification
                .states
                .iter()
                .find(|baseline| baseline.viewport.width == state.viewport.width)
            else {
                continue;
            };
            let roots = roots(state, baseline);
            if roots.is_empty() {
                continue;
            }
            let baseline_paths: HashSet<_> = baseline
                .nodes
                .iter()
                .map(|node| node.path.as_str())
                .collect();
            state.nodes.retain(|node| {
                if baseline_paths.contains(node.path.as_str()) {
                    return true;
                }
                if roots
                    .iter()
                    .any(|root| node.path == *root || descendant(&node.path, root))
                {
                    return true;
                }
                false
            });
        }
    }
}

pub fn paths(
    states: &[crate::model::PageState],
    baselines: &[crate::model::PageState],
) -> HashSet<String> {
    states
        .iter()
        .filter_map(|state| {
            baselines
                .iter()
                .find(|baseline| baseline.viewport.width == state.viewport.width)
                .map(|baseline| (state, roots(state, baseline)))
        })
        .flat_map(|(state, roots)| {
            state
                .nodes
                .iter()
                .filter(move |node| {
                    roots
                        .iter()
                        .any(|root| node.path == *root || descendant(&node.path, root))
                })
                .map(|node| node.path.clone())
        })
        .collect()
}

pub fn roots(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
) -> HashSet<String> {
    let baseline_paths: HashSet<_> = baseline
        .nodes
        .iter()
        .filter(|node| node.rect.width > 0.0 && node.rect.height > 0.0)
        .map(|node| node.path.as_str())
        .collect();
    let nodes: HashMap<_, _> = state
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    let indexes: HashMap<_, _> = state
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.path.as_str(), index))
        .collect();
    state
        .nodes
        .iter()
        .filter(|node| visible_action(node))
        .filter_map(|node| surface_root(node, &nodes, &baseline_paths))
        .max_by_key(|root| indexes.get(root.as_str()).copied().unwrap_or_default())
        .into_iter()
        .collect()
}

fn visible_action(node: &Node) -> bool {
    node.rect.width > 0.0
        && node.rect.height > 0.0
        && matches!(node.text.trim(), "Pin" | "Delete" | "Duplicate")
}

fn surface_root(
    node: &Node,
    nodes: &HashMap<&str, &Node>,
    baseline: &HashSet<&str>,
) -> Option<String> {
    let mut current = node;
    while let Some(parent) = current.parent.as_deref() {
        let Some(parent_node) = nodes.get(parent) else {
            break;
        };
        if parent_node
            .style
            .get("position")
            .is_some_and(|value| matches!(value.as_str(), "absolute" | "fixed"))
        {
            return Some(parent_node.path.clone());
        }
        if baseline.contains(parent) {
            break;
        }
        current = parent_node;
    }
    (!baseline.contains(current.path.as_str())).then(|| current.path.clone())
}

fn descendant(path: &str, root: &str) -> bool {
    path.strip_prefix(root)
        .is_some_and(|suffix| suffix.starts_with('>'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Interaction, PageState, Rect, Viewport};

    fn node(path: &str, parent: Option<&str>, text: &str, visible: bool) -> Node {
        Node {
            path: path.into(),
            parent: parent.map(str::to_string),
            tag: "div".into(),
            text: text.into(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: if visible { 20.0 } else { 0.0 },
                height: if visible { 20.0 } else { 0.0 },
            },
            style: Default::default(),
            before: None,
            after: None,
        }
    }

    #[test]
    fn removes_stale_hidden_overflow_surfaces() {
        let baseline = node("html>body", Some("html"), "", true);
        let state = PageState {
            url: String::new(),
            title: String::new(),
            viewport: Viewport::default(),
            nodes: vec![
                baseline.clone(),
                node(
                    "html>body>div:nth-of-type(1)",
                    Some("html>body"),
                    "Pin",
                    false,
                ),
                node(
                    "html>body>div:nth-of-type(2)",
                    Some("html>body"),
                    "Pin",
                    true,
                ),
            ],
            startup_nodes: Vec::new(),
            startup_delay_ms: 0,
            startup_duration_ms: 0,
            animations: Vec::new(),
            state_styles: Vec::new(),
            attribute_sequences: Vec::new(),
            css_rules: Vec::new(),
            asset_urls: Vec::new(),
            asset_data: Default::default(),
        };
        let mut specification = Specification {
            schema_version: 1,
            requested_url: String::new(),
            captured_url: String::new(),
            states: vec![PageState {
                nodes: vec![baseline],
                ..state.clone()
            }],
            interactions: vec![Interaction {
                trigger_path: String::new(),
                trigger_tag: "button".into(),
                trigger_label: "More options".into(),
                focused_path: None,
                states: vec![state],
            }],
        };
        normalize(&mut specification);
        assert_eq!(specification.interactions[0].states[0].nodes.len(), 2);
        assert!(
            specification.interactions[0].states[0].nodes[1]
                .path
                .ends_with("div:nth-of-type(2)")
        );
    }
}
