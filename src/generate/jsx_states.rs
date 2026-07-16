use super::{interactions, jsx_variants, structural_tree, tree};
use crate::model::Specification;
use std::collections::BTreeMap;

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
                let handlers = interactions::state_handlers(interaction, state);
                let page = jsx_variants::page(state, &components, assets, &handlers);
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
