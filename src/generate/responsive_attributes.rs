use crate::model::PageState;
use std::collections::{BTreeMap, BTreeSet, HashMap};

const EXCLUDED: &[&str] = &["class", "style"];

pub struct Javascript {
    pub paths: String,
    pub values: String,
    pub viewports: String,
}

pub fn javascript(states: &[PageState], canonical: usize) -> Javascript {
    let Some(base) = states.get(canonical) else {
        return Javascript {
            paths: "[]".into(),
            values: "[]".into(),
            viewports: "[]".into(),
        };
    };
    let base_nodes = base
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<HashMap<_, _>>();
    let mut varying = BTreeMap::<String, BTreeSet<String>>::new();
    for state in states {
        let nodes = state
            .nodes
            .iter()
            .map(|node| (node.path.as_str(), node))
            .collect::<HashMap<_, _>>();
        for (path, base_node) in &base_nodes {
            let Some(node) = nodes.get(path) else {
                continue;
            };
            for name in base_node
                .attributes
                .keys()
                .chain(node.attributes.keys())
                .filter(|name| !EXCLUDED.contains(&name.as_str()))
            {
                if base_node.attributes.get(name) != node.attributes.get(name) {
                    varying
                        .entry((*path).to_string())
                        .or_default()
                        .insert(name.clone());
                }
            }
        }
    }
    let paths = varying.keys().cloned().collect::<Vec<_>>();
    let path_indexes = paths
        .iter()
        .enumerate()
        .map(|(index, path)| (path.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut values = Vec::new();
    let mut value_indexes = HashMap::<String, usize>::new();
    let viewports = states
        .iter()
        .map(|state| {
            let nodes = state
                .nodes
                .iter()
                .map(|node| (node.path.as_str(), node))
                .collect::<HashMap<_, _>>();
            varying
                .iter()
                .filter_map(|(path, names)| {
                    let node = nodes.get(path.as_str())?;
                    let attributes = names
                        .iter()
                        .map(|name| serde_json::json!([name, node.attributes.get(name)]))
                        .collect::<Vec<_>>();
                    let serialized = serde_json::to_string(&attributes).unwrap();
                    let next = values.len();
                    let value_index = *value_indexes.entry(serialized).or_insert_with(|| {
                        values.push(attributes);
                        next
                    });
                    Some(serde_json::json!([
                        path_indexes[path.as_str()],
                        value_index
                    ]))
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    Javascript {
        paths: serde_json::to_string(&paths).unwrap(),
        values: serde_json::to_string(&values).unwrap(),
        viewports: serde_json::to_string(&viewports).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_each_responsive_attribute_value() {
        let specification = crate::generate::project_test_support::specification();
        let mut states = specification.states;
        let path = states[0].nodes[3].path.clone();
        states[0].nodes[3]
            .attributes
            .insert("viewBox".into(), "0 0 230 180".into());
        states[1].nodes[3]
            .attributes
            .insert("viewBox".into(), "0 0 310 180".into());

        let output = javascript(&states, 1);

        assert!(output.paths.contains(&path));
        assert!(output.values.contains("0 0 230 180"));
        assert!(output.values.contains("0 0 310 180"));
        assert!(output.viewports.contains("[0,"));
    }
}
