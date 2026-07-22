use crate::model::PageState;
use std::collections::{HashMap, HashSet};

pub fn surface_roots(state: &PageState, baseline: &PageState) -> HashSet<String> {
    let baseline_nodes = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<HashMap<_, _>>();
    let baseline_paths = baseline_nodes.keys().copied().collect::<HashSet<_>>();
    let added = state
        .nodes
        .iter()
        .filter(|node| {
            node.tag != "#text"
                && !baseline_paths.contains(node.path.as_str())
                && node
                    .parent
                    .as_deref()
                    .is_some_and(|parent| baseline_paths.contains(parent))
        })
        .map(|node| node.path.clone())
        .collect::<HashSet<_>>();
    let mut roots = added.clone();
    roots.extend(
        state
            .nodes
            .iter()
            .filter(|node| added.contains(&node.path))
            .filter_map(|node| node.parent.clone()),
    );
    roots.extend(
        state
            .nodes
            .iter()
            .filter(|node| matches!(node.tag.as_str(), "textarea" | "input"))
            .filter_map(|node| node.parent.clone()),
    );
    roots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Node, Rect};

    fn node(path: &str, parent: Option<&str>, width: f64) -> Node {
        Node {
            path: path.into(),
            parent: parent.map(str::to_owned),
            tag: "div".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width,
                height: 20.0,
            },
            style: Default::default(),
            before: None,
            after: None,
        }
    }

    #[test]
    fn keeps_inserted_controls_and_their_direct_parent_as_surfaces() {
        let container = "html>body>div:nth-of-type(1)";
        let control = format!("{container}>textarea:nth-of-type(1)");
        let baseline = PageState {
            url: String::new(),
            title: String::new(),
            viewport: Default::default(),
            dom: Default::default(),
            capture_blockers: Vec::new(),
            nodes: vec![
                node("html", None, 100.0),
                node("html>body", Some("html"), 100.0),
                node(container, Some("html>body"), 100.0),
                node(&control, Some(container), 100.0),
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
        let action = format!("{container}>button:nth-of-type(1)");
        let mut active = baseline.clone();
        active.nodes[2].rect.width = 140.0;
        active.nodes[3].rect.height = 40.0;
        active.nodes.push(node(&action, Some(container), 20.0));

        let roots = surface_roots(&active, &baseline);

        assert_eq!(roots, HashSet::from([container.into(), action]));
        assert!(!roots.contains(&control));
    }
}
