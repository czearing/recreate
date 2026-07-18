use super::{attribute_sequences, interactions, jsx, jsx_variants, structural_tree, tree};
use crate::model::Specification;
use std::collections::{BTreeMap, BTreeSet};

pub fn interaction_states(
    specification: &Specification,
    base: &tree::Components,
    class_maps: &[Vec<BTreeMap<String, String>>],
    assets: &BTreeMap<String, String>,
) -> String {
    let imports = base
        .items
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut output = format!(
        "import React from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{ {imports} }} from './components/index.js';\nconst keyActivate=(event,action)=>{{if(event.key==='Enter'||event.key===' '){{event.preventDefault();action(event)}}}};\n{}\n",
        jsx_variants::selector()
    );
    for (index, interaction) in specification.interactions.iter().enumerate() {
        let Some(classes) = class_maps.get(index) else {
            continue;
        };
        let views = interaction
            .states
            .iter()
            .zip(classes)
            .enumerate()
            .map(|(state_index, (state, classes))| {
                let components = structural_tree::for_state(base, state, classes);
                let baseline = specification
                    .states
                    .iter()
                    .find(|baseline| baseline.viewport.width == state.viewport.width)
                    .unwrap_or(&specification.states[0]);
                let mut handlers = interactions::state_handlers(interaction, state, baseline);
                attribute_sequences::append_handlers(baseline, &mut handlers);
                let page = overlay(state, baseline, &components, assets, &handlers);
                format!(
                    "function Interaction{}View{state_index}({{onReset}}){{return {page}}}\n",
                    index + 1
                )
            })
            .collect::<String>();
        let names = (0..interaction.states.len())
            .map(|state_index| format!("Interaction{}View{state_index}", index + 1))
            .collect::<Vec<_>>()
            .join(",");
        let widths = jsx_variants::widths(&interaction.states);
        output.push_str(&format!(
            "{views}const interaction{}Views=[{names}];\nexport function Interaction{}({{width,onReset}}){{const View=interaction{}Views[selectViewport(width,[{widths}])];return <View onReset={{onReset}}/>}}\n",
            index + 1,
            index + 1,
            index + 1
        ));
    }
    output
}

fn overlay(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
) -> String {
    let baseline_paths = baseline
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .collect::<BTreeSet<_>>();
    let roots = state
        .nodes
        .iter()
        .filter(|node| {
            !baseline_paths.contains(node.path.as_str())
                && node
                    .parent
                    .as_deref()
                    .is_none_or(|parent| baseline_paths.contains(parent))
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
