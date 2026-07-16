use super::{jsx, tree};
use crate::model::{PageState, Specification};
use std::collections::BTreeMap;

pub fn interaction_states(
    specification: &Specification,
    base: &tree::Components,
    class_maps: &[BTreeMap<String, String>],
    assets: &BTreeMap<String, String>,
) -> String {
    let imports = base
        .items
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut output = format!(
        "import React from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{ {imports} }} from './components/index.js';\n"
    );
    for (index, interaction) in specification.interactions.iter().enumerate() {
        let (Some(state), Some(classes)) = (interaction.states.first(), class_maps.get(index))
        else {
            continue;
        };
        let components = tree::for_state(base, state, classes);
        let handlers = BTreeMap::from([(
            interaction.trigger_path.clone(),
            "onClick={onReset}".to_string(),
        )]);
        let (content, portal) = page_parts(state, &components, assets, &handlers);
        output.push_str(&format!(
            "export function Interaction{}({{onReset}}){{return <>{}{}</>}}\n",
            index + 1,
            content,
            portal
        ));
    }
    output
}

fn page_parts(
    state: &PageState,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
) -> (String, String) {
    let body = state
        .nodes
        .iter()
        .find(|node| node.tag == "body")
        .map(|node| node.path.as_str())
        .unwrap_or("html");
    let root = state
        .nodes
        .iter()
        .find(|node| {
            node.attributes
                .get("id")
                .is_some_and(|value| value == "root")
        })
        .map(|node| node.path.as_str())
        .unwrap_or(body);
    let content = components
        .children
        .get(root)
        .into_iter()
        .flatten()
        .map(|path| jsx::render(path, components, assets, 2, true, handlers))
        .collect();
    let portal_nodes = components
        .children
        .get(body)
        .into_iter()
        .flatten()
        .filter(|path| path.as_str() != root)
        .map(|path| jsx::render(path, components, assets, 2, true, handlers))
        .collect::<String>();
    let portal = if portal_nodes.is_empty() {
        String::new()
    } else {
        format!("{{createPortal(<>{portal_nodes}</>,document.body)}}")
    };
    (content, portal)
}
