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
        "import React,{{useLayoutEffect}} from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{ {imports} }} from './components/index.js';\nconst keyActivate=(event,action)=>{{if(event.key==='Enter'||event.key===' '){{event.preventDefault();action(event)}}}};\nfunction textNode(path){{const match=path.match(/^(.*)>#text\\((\\d+)\\)$/);if(!match)return null;const parent=document.querySelector(match[1]);return parent?[...parent.childNodes].filter(node=>node.nodeType===3&&node.textContent.trim())[Number(match[2])-1]||null:null}}\nfunction applySurface(entries,roots,hidden,styles,texts,attributes,detach){{const restore=[];const restoreHidden=[];const restoreStyles=[];const restoreTexts=[];const restoreAttributes=[];for(const[path,className]of entries){{const element=document.querySelector(path);if(!element)continue;restore.push([element,element.getAttribute('class')]);element.setAttribute('class',className)}}for(const[path,values]of styles){{const element=document.querySelector(path);if(!element)continue;for(const[name,value]of values){{restoreStyles.push([element,name,element.style.getPropertyValue(name),element.style.getPropertyPriority(name)]);element.style.setProperty(name,value)}}}}for(const[path,values]of attributes){{const element=document.querySelector(path);if(!element)continue;for(const[name,value]of values){{restoreAttributes.push([element,name,element.getAttribute(name)]);value===null?element.removeAttribute(name):element.setAttribute(name,value)}}}}for(const[path,value]of texts){{const node=textNode(path);if(!node)continue;restoreTexts.push([node,node.nodeValue]);node.nodeValue=value}}for(const path of hidden){{const element=document.querySelector(path);if(!element)continue;if(detach&&element.parentNode){{restoreHidden.push([element,element.parentNode,element.nextSibling,null]);element.remove()}}else{{restoreHidden.push([element,null,null,element.style.display]);element.style.display='none'}}}}for(const path of roots){{const element=document.querySelector(path);if(element)element.dataset.recreateSurface='true'}}return()=>{{for(const[node,value]of restoreTexts)node.nodeValue=value;for(const[element,name,value]of restoreAttributes)value===null?element.removeAttribute(name):element.setAttribute(name,value);for(const[element,name,value,priority]of restoreStyles)value?element.style.setProperty(name,value,priority):element.style.removeProperty(name);for(const[element,className]of restore)className===null?element.removeAttribute('class'):element.setAttribute('class',className);for(const[element,parent,next,display]of restoreHidden)parent?parent.insertBefore(element,next):element.style.display=display;for(const path of roots)document.querySelector(path)?.removeAttribute('data-recreate-surface')}}}}\nfunction ExistingSurface({{entries,roots,hidden,styles,texts,attributes,detach}}){{useLayoutEffect(()=>{{let restore=applySurface(entries,roots,hidden,styles,texts,attributes,detach);const refresh=()=>{{restore();restore=applySurface(entries,roots,hidden,styles,texts,attributes,detach)}};window.addEventListener('recreate-surface-inserted',refresh);return()=>{{window.removeEventListener('recreate-surface-inserted',refresh);restore()}}}},[entries,roots,hidden,styles,texts,attributes,detach]);return null}}\nfunction ReplacementSurface({{path,className,children}}){{const[target,setTarget]=React.useState(null);useLayoutEffect(()=>{{const existing=document.querySelector(path);if(!existing)return;const previousClass=existing.getAttribute('class');const previousChildren=Array.from(existing.childNodes);existing.replaceChildren();existing.setAttribute('class',className);existing.dataset.recreateSurface='true';setTarget(existing);return()=>{{setTarget(null);previousClass===null?existing.removeAttribute('class'):existing.setAttribute('class',previousClass);existing.removeAttribute('data-recreate-surface');existing.replaceChildren(...previousChildren)}}}},[path,className]);return target?createPortal(children,target):null}}\nfunction InsertedSurface({{parentPath,beforePath,children}}){{const target=document.querySelector(parentPath);const attach=React.useCallback(inserted=>{{if(!inserted||!target||!beforePath)return;const before=document.querySelector(beforePath);if(before?.parentElement===target&&inserted.parentElement===target&&before!==inserted)target.insertBefore(inserted,before);queueMicrotask(()=>window.dispatchEvent(new Event('recreate-surface-inserted')))}},[target,beforePath]);return target?createPortal(React.cloneElement(React.Children.only(children),{{ref:attach}}),target):null}}\nfunction AnchoredSurface({{trigger,children}}){{const wrapper=React.useRef(null);useLayoutEffect(()=>{{const active=document.querySelector('[data-recreate-active=\"true\"]');const fallback=document.querySelector(`[data-recreate-trigger=\"${{trigger}}\"]`);const surface=wrapper.current?.firstElementChild;if(!active||!fallback||!surface)return;const current=surface.style.translate;const a=active.getBoundingClientRect();const b=fallback.getBoundingClientRect();surface.style.translate=`${{a.right-b.right}}px ${{a.bottom-b.bottom}}px`;return()=>{{surface.style.translate=current}}}},[trigger]);return createPortal(<div ref={{wrapper}} className=\"recreateAnchoredSurface\">{{children}}</div>,document.body)}}\n{}\n",
        jsx_variants::selector()
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
                let surface_roots = if full_replacement {
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
                    newly_visible_roots(state, baseline)
                };
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
                let mut handlers = interactions::state_handlers(interaction, state, baseline);
                attribute_sequences::append_handlers(baseline, &mut handlers);
                let changed_activation = if interactions::text_entry_interaction(interaction) {
                    existing_surface(
                        state,
                        baseline,
                        &components,
                        &Default::default(),
                        &Default::default(),
                        &std::collections::HashSet::from(["html".to_string()]),
                        &[],
                    )
                } else if !shared_surface && !surface_roots.is_empty() {
                    String::new()
                } else {
                    let changed = changed_existing_paths(state, baseline, &surface_roots);
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
                } else if state_control && surface_roots.is_empty() {
                    String::new()
                } else {
                    overlay(
                        state,
                        baseline,
                        &components,
                        assets,
                        &handlers,
                        (!surface_roots.is_empty()).then_some(&surface_roots),
                        !interactions::text_entry_interaction(interaction),
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

fn newly_visible_roots(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
) -> std::collections::HashSet<String> {
    let baseline_by_path = baseline
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
                && !baseline_by_path
                    .get(node.path.as_str())
                    .is_some_and(|baseline_node| {
                        visible(baseline_node)
                            && compatible_surface(node, baseline_node)
                            && compatible_children(state, baseline, &node.path)
                    })
        })
        .filter_map(|node| {
            let mut root = node;
            while let Some(parent) = root
                .parent
                .as_deref()
                .and_then(|path| nodes.get(path).copied())
            {
                if parent
                    .style
                    .get("position")
                    .is_some_and(|value| matches!(value.as_str(), "absolute" | "fixed"))
                {
                    root = parent;
                    break;
                }
                if baseline_by_path
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
    if roots.iter().any(|root| {
        nodes.get(root.as_str()).is_some_and(|node| {
            node.style
                .get("position")
                .is_some_and(|position| position == "fixed")
        })
    }) {
        roots.retain(|root| {
            nodes.get(root.as_str()).is_some_and(|node| {
                node.style
                    .get("position")
                    .is_some_and(|position| position == "fixed")
            })
        });
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

fn compatible_surface(state: &crate::model::Node, baseline: &crate::model::Node) -> bool {
    state.tag == baseline.tag && state.style.get("position") == baseline.style.get("position")
}

fn compatible_children(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
    path: &str,
) -> bool {
    let signature = |nodes: &[crate::model::Node]| {
        nodes
            .iter()
            .filter(|node| node.parent.as_deref() == Some(path))
            .map(|node| (node.tag.clone(), node.attributes.get("role").cloned()))
            .collect::<Vec<_>>()
    };
    signature(&state.nodes) == signature(&baseline.nodes)
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
    include_changed_existing: bool,
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
                    .is_some_and(|(state_node, baseline_node)| {
                        compatible_surface(state_node, baseline_node)
                            && (!include_changed_existing
                                || compatible_children(state, baseline, root))
                    })
            })
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let structural = surface_roots
            .iter()
            .filter(|root| !existing.contains(*root))
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let replacements = structural
            .iter()
            .filter(|root| {
                baseline_paths.contains(root.as_str()) && !shifted_insertion(root, state, baseline)
            })
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let mut activated = existing.clone();
        let shifted = structural
            .iter()
            .filter(|root| shifted_insertion(root, state, baseline))
            .filter_map(|root| next_sibling_path(root).map(|state_root| (state_root, root.clone())))
            .collect::<Vec<_>>();
        activated.retain(|path| {
            !shifted
                .iter()
                .any(|(state_root, _)| path == state_root || descendant_of(path, state_root))
        });
        if include_changed_existing {
            let mut changed = changed_existing_paths(state, baseline, &structural);
            changed.retain(|path| {
                !shifted
                    .iter()
                    .any(|(state_root, _)| path == state_root || descendant_of(path, state_root))
            });
            activated.extend(changed);
        }
        let mut delta_roots = activated.clone();
        delta_roots.extend(shifted.iter().map(|(state_root, _)| state_root.clone()));
        let activation = existing_surface(
            state,
            baseline,
            components,
            &activated,
            &existing,
            &delta_roots,
            &shifted,
        );
        let added = state
            .nodes
            .iter()
            .filter(|node| structural.contains(&node.path) && !replacements.contains(&node.path))
            .collect();
        let added_portals = inserted_surfaces(added, state, baseline, components, assets, handlers);
        let replacement_portals =
            replacement_surfaces(state, components, assets, handlers, &replacements);
        if !activation.is_empty() || !replacement_portals.is_empty() || !added_portals.is_empty() {
            return format!("<>{added_portals}{replacement_portals}{activation}</>");
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
        .filter_map(|node| {
            let previous = baseline.get(node.path.as_str())?;
            if node.tag == "#text" {
                (node.text != previous.text)
                    .then(|| node.parent.clone())
                    .flatten()
            } else {
                (node.style != previous.style
                    || node.attributes != previous.attributes
                    || node.text != previous.text)
                    .then(|| node.path.clone())
            }
        })
        .collect()
}

fn existing_surface(
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
    components: &tree::Components,
    roots: &std::collections::HashSet<String>,
    marked_roots: &std::collections::HashSet<String>,
    delta_roots: &std::collections::HashSet<String>,
    shifted: &[(String, String)],
) -> String {
    if roots.is_empty() && delta_roots.is_empty() {
        return String::new();
    }
    let entries = state
        .nodes
        .iter()
        .filter(|node| {
            !shifted.iter().any(|(state_root, _)| {
                node.path == *state_root || descendant_of(&node.path, state_root)
            })
        })
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
    let state_paths = state
        .nodes
        .iter()
        .map(|node| node.path.as_str())
        .collect::<BTreeSet<_>>();
    let mut hidden = baseline
        .nodes
        .iter()
        .filter(|node| node.tag != "#text" && !state_paths.contains(node.path.as_str()))
        .filter(|node| {
            roots.iter().any(|root| {
                node.path
                    .strip_prefix(root)
                    .is_some_and(|suffix| suffix.starts_with('>'))
            })
        })
        .map(|node| node.path.clone())
        .collect::<BTreeSet<_>>();
    let hidden_paths = hidden.clone();
    hidden.retain(|path| {
        !hidden_paths.iter().any(|parent| {
            parent != path
                && path
                    .strip_prefix(parent)
                    .is_some_and(|suffix| suffix.starts_with('>'))
        })
    });
    let baseline_nodes = baseline
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();
    let baseline_path = |path: &str| {
        shifted
            .iter()
            .find_map(|(state_root, baseline_root)| {
                path.strip_prefix(state_root)
                    .filter(|suffix| suffix.is_empty() || suffix.starts_with('>'))
                    .map(|suffix| format!("{baseline_root}{suffix}"))
            })
            .unwrap_or_else(|| path.to_string())
    };
    let styles = state
        .nodes
        .iter()
        .filter(|node| node.tag != "#text" && contained_by(delta_roots, &node.path))
        .filter_map(|node| {
            let path = baseline_path(&node.path);
            let baseline = baseline_nodes.get(path.as_str())?;
            let shifted_node = path != node.path;
            let changed = node
                .style
                .iter()
                .filter(|(name, value)| {
                    baseline.style.get(*name) != Some(*value)
                        || shifted_node && state_sensitive_property(name)
                })
                .collect::<Vec<_>>();
            (!changed.is_empty()).then_some((&node.path, changed))
        })
        .collect::<Vec<_>>();
    let texts = state
        .nodes
        .iter()
        .filter(|node| node.tag == "#text" && contained_by(delta_roots, &node.path))
        .filter_map(|node| {
            let path = baseline_path(&node.path);
            let baseline = baseline_nodes.get(path.as_str());
            (baseline.is_some_and(|baseline| node.text != baseline.text)
                || baseline.is_none() && contained_by(marked_roots, &node.path))
            .then_some((&node.path, &node.text))
        })
        .collect::<Vec<_>>();
    let attributes = state
        .nodes
        .iter()
        .filter(|node| node.tag != "#text" && contained_by(delta_roots, &node.path))
        .filter_map(|node| {
            let path = baseline_path(&node.path);
            let baseline = baseline_nodes.get(path.as_str())?;
            let names = node
                .attributes
                .keys()
                .chain(baseline.attributes.keys())
                .filter(|name| !matches!(name.as_str(), "class" | "style"))
                .collect::<BTreeSet<_>>();
            let changed = names
                .into_iter()
                .filter(|name| node.attributes.get(*name) != baseline.attributes.get(*name))
                .map(|name| (name, node.attributes.get(name)))
                .collect::<Vec<_>>();
            (!changed.is_empty()).then_some((&node.path, changed))
        })
        .collect::<Vec<_>>();
    let detach = marked_roots.iter().any(|root| {
        state.nodes.iter().any(|node| {
            matches!(node.tag.as_str(), "textarea" | "input")
                && (node.path == *root || descendant_of(&node.path, root))
        })
    });
    format!(
        "<ExistingSurface entries={{{}}} roots={{{}}} hidden={{{}}} styles={{{}}} texts={{{}}} attributes={{{}}} detach={{{detach}}}/>",
        serde_json::to_string(&entries).expect("surface classes should serialize"),
        serde_json::to_string(marked_roots).expect("surface roots should serialize"),
        serde_json::to_string(&hidden).expect("hidden paths should serialize"),
        serde_json::to_string(&styles).expect("surface styles should serialize"),
        serde_json::to_string(&texts).expect("surface text should serialize"),
        serde_json::to_string(&attributes).expect("surface attributes should serialize")
    )
}

fn contained_by(roots: &std::collections::HashSet<String>, path: &str) -> bool {
    roots
        .iter()
        .any(|root| path == root || descendant_of(path, root))
}

fn descendant_of(path: &str, root: &str) -> bool {
    path.strip_prefix(root)
        .is_some_and(|suffix| suffix.starts_with('>'))
}

fn state_sensitive_property(name: &str) -> bool {
    name == "animation"
        || name.starts_with("animation-")
        || matches!(
            name,
            "-webkit-text-fill-color"
                | "background-color"
                | "color"
                | "filter"
                | "opacity"
                | "pointer-events"
                | "transform"
                | "visibility"
                | "z-index"
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
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
    components: &tree::Components,
    assets: &BTreeMap<String, String>,
    handlers: &BTreeMap<String, String>,
) -> String {
    roots
        .into_iter()
        .filter_map(|node| {
            let parent = node.parent.as_ref()?;
            let rendered = jsx::render(&node.path, components, assets, 0, true, handlers);
            let before = if shifted_insertion(&node.path, state, baseline) {
                serde_json::to_string(&node.path).unwrap()
            } else {
                "null".into()
            };
            Some(format!(
                "<InsertedSurface parentPath={{{}}} beforePath={{{before}}}>{rendered}</InsertedSurface>",
                serde_json::to_string(parent).expect("surface parent path should serialize"),
            ))
        })
        .collect()
}

fn shifted_insertion(
    path: &str,
    state: &crate::model::PageState,
    baseline: &crate::model::PageState,
) -> bool {
    let Some(next) = next_sibling_path(path) else {
        return false;
    };
    let baseline_node = baseline.nodes.iter().find(|node| node.path == path);
    let shifted = state.nodes.iter().find(|node| node.path == next);
    baseline_node
        .zip(shifted)
        .is_some_and(|(baseline_node, shifted)| {
            baseline_node.tag == shifted.tag
                && baseline_node.attributes == shifted.attributes
                && child_signature(baseline, path) == child_signature(state, &next)
        })
}

fn next_sibling_path(path: &str) -> Option<String> {
    let (prefix, suffix) = path.rsplit_once(":nth-of-type(")?;
    let index = suffix.strip_suffix(')')?.parse::<usize>().ok()?;
    Some(format!("{prefix}:nth-of-type({})", index + 1))
}

fn child_signature(state: &crate::model::PageState, path: &str) -> Vec<(String, Option<String>)> {
    state
        .nodes
        .iter()
        .filter(|node| node.parent.as_deref() == Some(path))
        .map(|node| (node.tag.clone(), node.attributes.get("role").cloned()))
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
            format!("<AnchoredSurface trigger={{{trigger}}}>{content}</AnchoredSurface>")
        })
        .collect::<String>();
    format!("<>{portals}</>")
}
