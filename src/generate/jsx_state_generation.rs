use super::{
    attribute_sequences, interactions, jsx_state_changes::*, jsx_state_existing::*,
    jsx_state_overlay::*, jsx_state_portals::*, jsx_state_roots::*, jsx_variants, structural_tree,
    tree,
};
use crate::model::Specification;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// The interaction-state module is authored as real JavaScript under
/// `templates/`, so a syntax error is caught by `node --check` rather than by the
/// browser gate. `state_anchored.jsx` carries the only JSX in the template and is
/// therefore outside that parser's grammar.
const STATE_TEMPLATE: &str = concat!(
    include_str!("templates/state_surfaces.mjs"),
    include_str!("templates/state_anchored.jsx"),
);

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
    let selector = jsx_variants::selector();
    let mut output = super::jsx_app::fill(
        STATE_TEMPLATE,
        &[
            ("__RECREATE_IMPORTS__", &imports),
            ("\"__RECREATE_POSITIONAL__\"", selector),
        ],
    );
    output = output
        .replace(
            "const stableRoots=roots.map(path=>{const element=document.querySelector(path);if(!element)return null;return[element,element.getBoundingClientRect().height,element.style.getPropertyValue('min-height'),element.style.getPropertyPriority('min-height')]}).filter(Boolean);",
            "",
        )
        .replace(
            "for(const[element,height]of stableRoots)if(element.getBoundingClientRect().height+1<height)element.style.setProperty('min-height',`${height}px`,'important');",
            "",
        )
        .replace(
            "for(const[element,,value,priority]of stableRoots)value?element.style.setProperty('min-height',value,priority):element.style.removeProperty('min-height');",
            "",
        )
        .replace(
            ",minHeight:[existing.style.getPropertyValue('min-height'),existing.style.getPropertyPriority('min-height')],height:existing.getBoundingClientRect().height",
            "",
        )
        .replace(
            "if(baseline.height>0)existing.style.setProperty('min-height',`${baseline.height}px`);",
            "",
        )
        .replace(
            "baseline.minHeight[0]?existing.style.setProperty('min-height',baseline.minHeight[0],baseline.minHeight[1]):existing.style.removeProperty('min-height');",
            "",
        )
        .replace(
            "const host=document.createElement('span');host.style.display='contents';host.dataset.recreateReplacement='true';",
            "",
        )
        .replace(
            "function ReplacementSurface({path,className,children}){const[target,setTarget]=React.useState(null);useLayoutEffect(()=>{",
            "function ReplacementSurface({path,className,children}){const[host]=React.useState(()=>{const host=document.createElement('span');host.style.display='contents';host.dataset.recreateReplacement='true';return host});useLayoutEffect(()=>{",
        )
        .replace("setTarget(host);", "")
        .replace("return()=>{setTarget(null);queueMicrotask", "return()=>{queueMicrotask")
        .replace(
            "}},[path,className]);return target?createPortal(children,target):null}",
            "}},[path,className,host]);return createPortal(children,host)}",
        );
    let interactions = specification
        .interactions
        .par_iter()
        .enumerate()
        .map(|(index, interaction)| {
        if !interactions::rendered(interaction, &specification.states) {
            return format!(
                "export function Interaction{}(){{return null}}\n",
                index + 1
            );
        }
        let Some(classes) = class_maps.get(index) else {
            return String::new();
        };
        let shared_surface = interactions::shared_trigger(interaction, &specification.states);
        let fallback_surface = if shared_surface {
            interaction
                .states
                .iter()
                .enumerate()
                .find_map(|(state_index, state)| {
                    let baseline = specification
                        .states
                        .iter()
                        .find(|baseline| baseline.viewport.width == state.viewport.width)?;
                    let roots = crate::interaction_surface::roots(state, baseline);
                    (!roots.is_empty()).then_some((state_index, roots))
                })
        } else {
            None
        };
        let state_control = interactions::state_control(interaction, &specification.states);
        let views = interaction
            .states
            .iter()
            .zip(classes)
            .enumerate()
            .map(|(state_index, (state, current_classes))| {
                let baseline = specification
                    .states
                    .iter()
                    .find(|baseline| baseline.viewport.width == state.viewport.width)
                    .unwrap_or(&specification.states[0]);
                let full_replacement = false;
                let mut surface_roots = if full_replacement {
                    Default::default()
                } else if shared_surface {
                    let roots = crate::interaction_surface::roots(state, baseline);
                    if roots.is_empty() {
                        let paths = state
                            .nodes
                            .iter()
                            .map(|node| node.path.as_str())
                            .collect::<BTreeSet<_>>();
                        fallback_surface
                            .as_ref()
                            .map(|(_, roots)| {
                                roots
                                    .iter()
                                    .filter(|root| paths.contains(root.as_str()))
                                    .cloned()
                                    .collect()
                            })
                            .unwrap_or_default()
                    } else {
                        roots
                    }
                } else if interactions::text_entry_interaction(interaction) {
                    super::jsx_text_entry::surface_roots(state, baseline)
                } else {
                    let mut roots = newly_visible_roots(state, baseline);
                    if state_control {
                        roots.extend(changed_structure_roots(state, baseline));
                    }
                    roots
                };
                if state_control {
                    let trigger_parent = interaction
                        .trigger_path
                        .rsplit_once('>')
                        .map(|(parent, _)| parent);
                    let trigger_y = state
                        .nodes
                        .iter()
                        .find(|node| node.path == interaction.trigger_path)
                        .map(|node| node.rect.y);
                    surface_roots.retain(|root| {
                        interaction.trigger_path != *root
                            && !descendant_of(&interaction.trigger_path, root)
                            && trigger_parent
                                .is_none_or(|parent| !descendant_of(root, parent))
                            && trigger_y.is_none_or(|trigger_y| {
                                state
                                    .nodes
                                    .iter()
                                    .find(|node| node.path == *root)
                                    .is_some_and(|node| node.rect.y >= trigger_y - 1.0)
                            })
                    });
                    let mut siblings = std::collections::HashMap::<String, Vec<String>>::new();
                    for root in &surface_roots {
                        if let Some((parent, _)) = root.rsplit_once('>') {
                            siblings
                                .entry(parent.to_string())
                                .or_default()
                                .push(root.clone());
                        }
                    }
                    for (parent, children) in siblings {
                        if children.len() < 2 {
                            continue;
                        }
                        surface_roots.retain(|root| !children.contains(root));
                        surface_roots.insert(parent);
                    }
                    let all_surface_roots = surface_roots.clone();
                    surface_roots.retain(|root| {
                        !all_surface_roots
                            .iter()
                            .any(|candidate| candidate != root && descendant_of(root, candidate))
                    });
                }
                let floating_surface = surface_roots.iter().any(|root| {
                    state
                        .nodes
                        .iter()
                        .find(|node| node.path == *root)
                        .and_then(|node| node.style.get("position"))
                        .is_some_and(|position| matches!(position.as_str(), "absolute" | "fixed"))
                });
                if shared_surface
                    && surface_roots.is_empty()
                    && let Some((fallback_index, fallback_roots)) = &fallback_surface
                {
                    let fallback_state = &interaction.states[*fallback_index];
                    let fallback_classes = &classes[*fallback_index];
                    let nodes = fallback_state
                        .nodes
                        .iter()
                        .filter(|node| {
                            fallback_roots.iter().any(|root| {
                                node.path == *root || node.path.starts_with(&format!("{root}>"))
                            })
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    let components = structural_tree::fragment_nodes(&nodes, fallback_classes);
                    let fallback_baseline = specification
                        .states
                        .iter()
                        .find(|baseline| {
                            baseline.viewport.width == fallback_state.viewport.width
                        })
                        .unwrap_or(&specification.states[0]);
                    let handlers = interactions::state_handlers(
                        specification,
                        index + 1,
                        interaction,
                        fallback_state,
                        fallback_baseline,
                    );
                    let roots = fallback_state
                        .nodes
                        .iter()
                        .filter(|node| fallback_roots.contains(&node.path))
                        .collect();
                    let page =
                        trigger_portals(roots, &components, assets, &handlers, index + 1);
                    return format!(
                        "function Interaction{}View{state_index}({{onReset}}){{return {page}}}\n",
                        index + 1
                    );
                }
                let components = if full_replacement {
                    structural_tree::fragment_nodes(&state.nodes, current_classes)
                } else if !surface_roots.is_empty() && shared_surface {
                    let nodes = state
                        .nodes
                        .iter()
                        .filter(|node| {
                            surface_roots.iter().any(|root| {
                                node.path == *root || node.path.starts_with(&format!("{root}>"))
                            })
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    structural_tree::fragment_nodes(&nodes, current_classes)
                } else {
                    structural_tree::for_state(base, state, current_classes)
                };
                let mut handlers = interactions::state_handlers(
                    specification,
                    index + 1,
                    interaction,
                    state,
                    baseline,
                );
                attribute_sequences::append_handlers(baseline, &mut handlers);
                let changed_activation = if shared_surface || floating_surface {
                    String::new()
                } else if interactions::text_entry_interaction(interaction) {
                    existing_surface(
                        state,
                        baseline,
                        &components,
                        &Default::default(),
                        &Default::default(),
                        &std::collections::HashSet::from(["html".to_string()]),
                        &[],
                    )
                } else if state_control || !shared_surface && !surface_roots.is_empty() {
                    String::new()
                } else {
                    let mut changed = changed_existing_paths(state, baseline, &surface_roots);
                    if state_control
                        && let Some((trigger_parent, _)) =
                            interaction.trigger_path.rsplit_once('>')
                    {
                        changed.retain(|path| {
                            path == trigger_parent || descendant_of(path, trigger_parent)
                        });
                    }
                    existing_surface(
                        state,
                        baseline,
                        &components,
                        &changed,
                        &Default::default(),
                        &changed,
                        &[],
                    )
                };
                let page = if full_replacement {
                    jsx_variants::page(state, &components, assets, &handlers)
                } else if shared_surface {
                    trigger_portals(
                        state
                            .nodes
                            .iter()
                            .filter(|node| surface_roots.contains(&node.path))
                            .collect(),
                        &components,
                        assets,
                        &handlers,
                        index + 1,
                    )
                } else if surface_roots.is_empty() {
                    String::new()
                } else {
                    overlay(
                        state,
                        baseline,
                        &components,
                        assets,
                        &handlers,
                        (!surface_roots.is_empty()).then_some(&surface_roots),
                        (
                            !interactions::text_entry_interaction(interaction)
                                && !state_control
                                && !floating_surface,
                            interactions::text_entry_interaction(interaction),
                        ),
                    )
                };
                format!(
                    "function Interaction{}View{state_index}({{onReset}}){{return <>{changed_activation}{page}</>}}\n",
                    index + 1
                )
            })
            .collect::<String>();
        let names = (0..interaction.states.len())
            .map(|state_index| format!("Interaction{}View{state_index}", index + 1))
            .collect::<Vec<_>>()
            .join(",");
        let widths = jsx_variants::widths(&interaction.states);
        format!(
            "{views}const interaction{}Views=[{names}];\nexport function Interaction{}({{width,onReset}}){{const View=interaction{}Views[selectViewport(width,[{widths}])];return <View onReset={{onReset}}/>}}\n",
            index + 1,
            index + 1,
            index + 1
        )
    })
    .collect::<Vec<_>>()
    .join("");
    output.push_str(&interactions);
    output
}
