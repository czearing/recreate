use crate::model::{Acceptance, Specification};
use anyhow::Result;
use std::{collections::BTreeMap, path::Path};

pub fn validate(specification: &Specification, out: &Path) -> Result<Acceptance> {
    let mut checks = BTreeMap::new();
    checks.insert("has_states".into(), !specification.states.is_empty());
    checks.insert(
        "all_states_have_nodes".into(),
        specification
            .states
            .iter()
            .all(|state| !state.nodes.is_empty()),
    );
    checks.insert(
        "all_states_match_source".into(),
        specification
            .states
            .iter()
            .all(|state| state.url == specification.captured_url),
    );
    checks.insert(
        "react_project_exists".into(),
        out.join("react/src/App.jsx").exists()
            && out.join("react/src/components/index.js").exists()
            && out.join("react/src/styles.css").exists(),
    );
    checks.insert(
        "unique_node_paths".into(),
        specification.states.iter().all(|state| {
            let paths: std::collections::HashSet<_> =
                state.nodes.iter().map(|node| &node.path).collect();
            paths.len() == state.nodes.len()
        }),
    );
    let mut counts = BTreeMap::new();
    counts.insert("states".into(), specification.states.len());
    counts.insert(
        "nodes".into(),
        specification
            .states
            .iter()
            .map(|state| state.nodes.len())
            .sum(),
    );
    counts.insert(
        "animations".into(),
        specification
            .states
            .iter()
            .map(|state| state.animations.len())
            .sum(),
    );
    counts.insert(
        "assets".into(),
        specification
            .states
            .iter()
            .flat_map(|state| &state.asset_urls)
            .collect::<std::collections::HashSet<_>>()
            .len(),
    );
    counts.insert("interactions".into(), specification.interactions.len());
    Ok(Acceptance {
        passed: checks.values().all(|value| *value),
        checks,
        counts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Specification;

    #[test]
    fn empty_specification_fails() {
        let directory = tempfile::tempdir().unwrap();
        let result = validate(
            &Specification {
                schema_version: 1,
                requested_url: String::new(),
                captured_url: String::new(),
                states: Vec::new(),
                interactions: Vec::new(),
            },
            directory.path(),
        )
        .unwrap();
        assert!(!result.passed);
    }
}
