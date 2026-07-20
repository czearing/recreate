use super::{attribute_sequences, interactions, jsx, jsx_variants, structural_tree, tree};
use crate::model::Specification;
use rayon::prelude::*;
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
        "import React,{{useLayoutEffect}} from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{ {imports} }} from './components/index.js';\nconst keyActivate=(event,action)=>{{if(event.key==='Enter'||event.key===' '){{event.preventDefault();action(event)}}}};\nfunction ExistingSurface({{entries,roots}}){{useLayoutEffect(()=>{{const restore=[];for(const[path,className]of entries){{const element=document.querySelector(path);if(!element)continue;restore.push([element,element.getAttribute('class')]);element.setAttribute('class',className)}}for(const path of roots){{const element=document.querySelector(path);if(element)element.dataset.recreateSurface='true'}}return()=>{{for(const[element,className]of restore)className===null?element.removeAttribute('class'):element.setAttribute('class',className);for(const path of roots)document.querySelector(path)?.removeAttribute('data-recreate-surface')}}}},[entries,roots]);return null}}\nfunction ReplacementSurface({{path,className,children}}){{const[target,setTarget]=React.useState(null);useLayoutEffect(()=>{{const existing=document.querySelector(path);if(!existing)return;const previousClass=existing.getAttribute('class');const previousChildren=Array.from(existing.childNodes);existing.replaceChildren();existing.setAttribute('class',className);existing.dataset.recreateSurface='true';setTarget(existing);return()=>{{setTarget(null);previousClass===null?existing.removeAttribute('class'):existing.setAttribute('class',previousClass);existing.removeAttribute('data-recreate-surface');existing.replaceChildren(...previousChildren)}}}},[path,className]);return target?createPortal(children,target):null}}\nfunction InsertedSurface({{parentPath,children}}){{const[target,setTarget]=React.useState(null);useLayoutEffect(()=>{{setTarget(document.querySelector(parentPath))}},[parentPath]);return target?createPortal(children,target):null}}\nfunction SuppressPortals(){{useLayoutEffect(()=>{{const entries=Array.from(document.querySelectorAll('body>[data-portal-node]')).map(element=>[element,element.nextSibling]);for(const[element]of entries)element.remove();return()=>{{for(const[element,next]of entries)document.body.insertBefore(element,next?.parentNode===document.body?next:null)}}}},[]);return null}}\n{}\n",
        jsx_variants::selector()
    );
    let interactions = specification
        .interactions
        .par_iter()
        .enumerate()
        .map(|(index, interaction)| {
        if !interactions::closable(interaction, &specification.states) {
            return format!(
                "export function Interaction{}(){{return null}}\n",
                index + 1
            );
        }
        let Some(classes) = class_maps.get(index) else {
            return String::new();
        };
        let overflow = interaction.trigger_label.eq_ignore_ascii_case("More options");
        let fallback_surface = if overflow {
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
                let full_replacement = state.nodes.len() * 4 < baseline.nodes.len() * 3
                    || structural_surface_replacement(state, baseline);
                let surface_roots = if full_replacement {
                    Default::default()
                } else if overflow {
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
                } else {
                    newly_visible_roots(state, baseline)
                };
                if overflow
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
                    let handlers =
                        interactions::state_handlers(interaction, fallback_state, fallback_baseline);
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
                } else if !surface_roots.is_empty() && overflow {
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
                let mut handlers = interactions::state_handlers(interaction, state, baseline);
                attribute_sequences::append_handlers(baseline, &mut handlers);
                let page = if full_replacement {
                    jsx_variants::page(state, &components, assets, &handlers)
                } else if overflow {
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
                } else {
                    overlay(
                        state,
                        baseline,
                        &components,
                        assets,
                        &handlers,
                        (!surface_roots.is_empty()).then_some(&surface_roots),
                    )
                };
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

fn newly_visible_roots(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
) -> std::collections::HashSet<String> {
    let baseline = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();
    let nodes = state
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();
    let mut roots = state
        .nodes
        .iter()
        .filter(|node| {
            visible_surface(node)
                && !baseline
                    .get(node.path.as_str())
                    .is_some_and(|baseline| compatible_surface(node, baseline))
        })
        .filter_map(|node| {
            let mut root = node;
            while let Some(parent) = root
                .parent
                .as_deref()
                .and_then(|path| nodes.get(path).copied())
            {
                if baseline
                    .get(parent.path.as_str())
                    .is_some_and(|node| visible(node))
                {
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
    if roots.len() > 1 {
        let counts = roots
            .iter()
            .map(|root| {
                let count = state
                    .nodes
                    .iter()
                    .filter(|node| {
                        node.path == **root
                            || node
                                .path
                                .strip_prefix(root.as_str())
                                .is_some_and(|suffix| suffix.starts_with('>'))
                    })
                    .count();
                (root.clone(), count)
            })
            .collect::<std::collections::HashMap<_, _>>();
        roots.retain(|root| counts.get(root).is_some_and(|count| *count >= 20));
    }
    roots
}

fn visible_surface(node: &crate::model::Node) -> bool {
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

pub(super) fn structural_surface_replacement(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
) -> bool {
    let baseline = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();
    state.nodes.iter().any(|node| {
        visible_surface(node)
            && baseline
                .get(node.path.as_str())
                .is_some_and(|baseline| !compatible_surface(node, baseline))
    })
}

fn compatible_surface(state: &crate::model::Node, baseline: &crate::model::Node) -> bool {
    visible(baseline)
        && state.tag == baseline.tag
        && state.style.get("position") == baseline.style.get("position")
}

fn visible(node: &crate::model::Node) -> bool {
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

fn overlay(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
    known_surface_roots: Option<&std::collections::HashSet<String>>,
) -> String {
    if let Some(surface_roots) = known_surface_roots {
        let baseline_paths = baseline
            .nodes
            .iter()
            .map(|node| node.path.as_str())
            .collect::<BTreeSet<_>>();
        let existing = surface_roots
            .iter()
            .filter(|root| {
                let state_node = state.nodes.iter().find(|node| node.path == root.as_str());
                let baseline_node = baseline
                    .nodes
                    .iter()
                    .find(|node| node.path == root.as_str());
                state_node
                    .zip(baseline_node)
                    .is_some_and(|(state, baseline)| compatible_surface(state, baseline))
            })
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let replacements = surface_roots
            .iter()
            .filter(|root| baseline_paths.contains(root.as_str()) && !existing.contains(*root))
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let mut activated = existing.clone();
        activated.extend(changed_existing_paths(state, baseline, &replacements));
        let activation = existing_surface(state, components, &activated);
        let added = state
            .nodes
            .iter()
            .filter(|node| {
                surface_roots.contains(&node.path)
                    && !existing.contains(&node.path)
                    && !replacements.contains(&node.path)
            })
            .collect();
        let added_portals = inserted_surfaces(added, components, assets, handlers);
        let replacement_portals =
            replacement_surfaces(state, components, assets, handlers, &replacements);
        if !activation.is_empty() || !replacement_portals.is_empty() {
            return format!(
                "<><SuppressPortals/>{activation}{replacement_portals}{added_portals}</>"
            );
        }
        return portals(
            state
                .nodes
                .iter()
                .filter(|node| surface_roots.contains(&node.path))
                .collect(),
            components,
            assets,
            handlers,
        );
    }
    let baseline_paths = baseline
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .collect::<BTreeSet<_>>();
    let surface_roots = crate::interaction_surface::roots(state, baseline);
    let roots = state
        .nodes
        .iter()
        .filter(|node| {
            surface_roots.contains(&node.path)
                || (!baseline_paths.contains(node.path.as_str())
                    && node
                        .parent
                        .as_deref()
                        .is_none_or(|parent| baseline_paths.contains(parent)))
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
    portals(roots, components, assets, handlers)
}

fn changed_existing_paths(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
    replacements: &std::collections::HashSet<String>,
) -> std::collections::HashSet<String> {
    let baseline = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();
    state
        .nodes
        .iter()
        .filter(|node| {
            !replacements.iter().any(|root| {
                node.path == *root
                    || node
                        .path
                        .strip_prefix(root)
                        .is_some_and(|suffix| suffix.starts_with('>'))
            })
        })
        .filter(|node| {
            baseline.get(node.path.as_str()).is_some_and(|baseline| {
                ["color", "background-color"]
                    .iter()
                    .any(|property| node.style.get(*property) != baseline.style.get(*property))
            })
        })
        .map(|node| node.path.clone())
        .collect()
}

fn existing_surface(
    state: &crate::model::PageState,
    components: &tree::Components,
    roots: &std::collections::HashSet<String>,
) -> String {
    if roots.is_empty() {
        return String::new();
    }
    let entries = state
        .nodes
        .iter()
        .filter(|node| {
            roots.iter().any(|root| {
                node.path == *root
                    || node
                        .path
                        .strip_prefix(root)
                        .is_some_and(|suffix| suffix.starts_with('>'))
            })
        })
        .filter_map(|node| {
            components
                .classes
                .get(&node.path)
                .map(|class| (&node.path, class))
        })
        .collect::<Vec<_>>();
    format!(
        "<ExistingSurface entries={{{}}} roots={{{}}}/>",
        serde_json::to_string(&entries).expect("surface classes should serialize"),
        serde_json::to_string(&roots).expect("surface roots should serialize")
    )
}

fn replacement_surfaces(
    state: &crate::model::PageState,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
    roots: &std::collections::HashSet<String>,
) -> String {
    state
        .nodes
        .iter()
        .filter(|node| roots.contains(&node.path))
        .map(|node| {
            let rendered = jsx::render_children(&node.path, components, assets, 0, handlers);
            let class = components
                .classes
                .get(&node.path)
                .expect("replacement surface should have a class");
            format!(
                "<ReplacementSurface path={{{}}} className={{{}}}>{rendered}</ReplacementSurface>",
                serde_json::to_string(&node.path).expect("surface path should serialize"),
                serde_json::to_string(class).expect("surface class should serialize")
            )
        })
        .collect()
}

fn inserted_surfaces(
    roots: Vec<&crate::model::Node>,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
) -> String {
    roots
        .into_iter()
        .filter_map(|node| {
            let parent = node.parent.as_ref()?;
            let rendered = jsx::render(&node.path, components, assets, 0, true, handlers);
            Some(format!(
                "<InsertedSurface parentPath={{{}}}>{rendered}</InsertedSurface>",
                serde_json::to_string(parent).expect("surface parent path should serialize")
            ))
        })
        .collect()
}

fn portals(
    roots: Vec<&crate::model::Node>,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
) -> String {
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

fn trigger_portals(
    roots: Vec<&crate::model::Node>,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
    trigger: usize,
) -> String {
    let portals = roots
        .into_iter()
        .map(|node| {
            let content = jsx::render(&node.path, components, assets, 2, true, handlers);
            format!(
                "{{createPortal(<>{content}</>,document.querySelector('[data-recreate-trigger=\"{trigger}\"]')?.parentElement||document.body)}}"
            )
        })
        .collect::<String>();
    format!("<>{portals}</>")
}
