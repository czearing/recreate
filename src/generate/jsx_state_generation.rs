use super::{
    attribute_sequences, interactions, jsx_state_changes::*, jsx_state_existing::*,
    jsx_state_overlay::*, jsx_state_portals::*, jsx_state_roots::*, jsx_variants, structural_tree,
    tree,
};
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
        "import React,{{useLayoutEffect}} from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{ {imports} }} from './components/index.js';\nconst keyActivate=(event,action)=>{{if(event.key==='Enter'||event.key===' '){{event.preventDefault();action(event)}}}};\nfunction textNode(path){{const match=path.match(/^(.*)>#text\\((\\d+)\\)$/);if(!match)return null;const parent=document.querySelector(match[1]);return parent?[...parent.childNodes].filter(node=>node.nodeType===3&&node.textContent.trim())[Number(match[2])-1]||null:null}}\nfunction applySurface(entries,roots,hidden,styles,texts,attributes,detach){{const restore=[];const restoreHidden=[];const restoreStyles=[];const restoreTexts=[];const restoreAttributes=[];const stableRoots=roots.map(path=>{{const element=document.querySelector(path);if(!element)return null;return[element,element.getBoundingClientRect().height,element.style.getPropertyValue('min-height'),element.style.getPropertyPriority('min-height')]}}).filter(Boolean);for(const[path,className]of entries){{const element=document.querySelector(path);if(!element)continue;restore.push([element,element.getAttribute('class')]);element.setAttribute('class',className)}}for(const[path,values]of styles){{const element=document.querySelector(path);if(!element)continue;for(const[name,value]of values){{restoreStyles.push([element,name,element.style.getPropertyValue(name),element.style.getPropertyPriority(name)]);element.style.setProperty(name,value,'important')}}}}for(const[path,values]of attributes){{const element=document.querySelector(path);if(!element)continue;for(const[name,value]of values){{restoreAttributes.push([element,name,element.getAttribute(name)]);value===null?element.removeAttribute(name):element.setAttribute(name,value)}}}}for(const[path,value]of texts){{const node=textNode(path);if(!node)continue;restoreTexts.push([node,node.nodeValue]);node.nodeValue=value}}for(const path of hidden){{const element=document.querySelector(path);if(!element)continue;if(detach&&element.parentNode){{restoreHidden.push([element,element.parentNode,element.nextSibling,null]);element.remove()}}else{{restoreHidden.push([element,null,null,element.style.display]);element.style.display='none'}}}}for(const[element,height]of stableRoots)if(element.getBoundingClientRect().height+1<height)element.style.setProperty('min-height',`${{height}}px`,'important');for(const path of roots){{const element=document.querySelector(path);if(element)element.dataset.recreateSurface='true'}}return()=>{{for(const[node,value]of restoreTexts)node.nodeValue=value;for(const[element,name,value]of restoreAttributes)value===null?element.removeAttribute(name):element.setAttribute(name,value);for(const[element,name,value,priority]of restoreStyles)value?element.style.setProperty(name,value,priority):element.style.removeProperty(name);for(const[element,className]of restore)className===null?element.removeAttribute('class'):element.setAttribute('class',className);for(const[element,parent,next,display]of restoreHidden)parent?parent.insertBefore(element,next):element.style.display=display;for(const[element,,value,priority]of stableRoots)value?element.style.setProperty('min-height',value,priority):element.style.removeProperty('min-height');for(const path of roots)document.querySelector(path)?.removeAttribute('data-recreate-surface')}}}}\nfunction ExistingSurface({{entries,roots,hidden,styles,texts,attributes,detach}}){{useLayoutEffect(()=>{{let restore=applySurface(entries,roots,hidden,styles,texts,attributes,detach);const refresh=()=>{{restore();restore=applySurface(entries,roots,hidden,styles,texts,attributes,detach)}};window.addEventListener('recreate-surface-inserted',refresh);return()=>{{window.removeEventListener('recreate-surface-inserted',refresh);restore()}}}},[entries,roots,hidden,styles,texts,attributes,detach]);return null}}\n                                function ReplacementSurface({{path,className,children}}){{const[target,setTarget]=React.useState(null);useLayoutEffect(()=>{{const existing=document.querySelector(path);if(!existing)return;const baseline=existing.__recreateBaseline??{{className:existing.getAttribute('class'),minHeight:[existing.style.getPropertyValue('min-height'),existing.style.getPropertyPriority('min-height')],height:existing.getBoundingClientRect().height,children:Array.from(existing.childNodes).map(node=>[node,node.nodeType===3?node.nodeValue:node.style?.display])}};existing.__recreateBaseline=baseline;const token={{}};existing.__recreateReplacement=token;const host=document.createElement('span');host.style.display='contents';host.dataset.recreateReplacement='true';for(const[node]of baseline.children)node.nodeType===3?node.nodeValue='':node.style.setProperty('display','none','important');existing.append(host);existing.setAttribute('class',className);if(baseline.height>0)existing.style.setProperty('min-height',`${{baseline.height}}px`);existing.dataset.recreateSurface='true';setTarget(host);return()=>{{setTarget(null);queueMicrotask(()=>{{host.remove();if(existing.__recreateReplacement!==token)return;delete existing.__recreateReplacement;delete existing.__recreateBaseline;baseline.className===null?existing.removeAttribute('class'):existing.setAttribute('class',baseline.className);baseline.minHeight[0]?existing.style.setProperty('min-height',baseline.minHeight[0],baseline.minHeight[1]):existing.style.removeProperty('min-height');existing.removeAttribute('data-recreate-surface');for(const[node,value]of baseline.children)node.nodeType===3?node.nodeValue=value:value?node.style.setProperty('display',value):node.style.removeProperty('display')}})}}}},[path,className]);return target?createPortal(children,target):null}}\n                function InsertedSurface({{parentPath,beforePath,floating=false,children}}){{const target=floating?document.body:document.querySelector(parentPath);const attach=React.useCallback(inserted=>{{if(!inserted||!target)return;if(beforePath&&!floating){{const before=document.querySelector(beforePath);if(before?.parentElement===target&&inserted.parentElement===target&&before!==inserted)target.insertBefore(inserted,before)}}queueMicrotask(()=>window.dispatchEvent(new Event('recreate-surface-inserted')))}},[target,beforePath,floating]);return target?createPortal(React.cloneElement(React.Children.only(children),{{ref:attach}}),target):null}}\nfunction AnchoredSurface({{trigger,children}}){{const wrapper=React.useRef(null);useLayoutEffect(()=>{{const active=document.querySelector('[data-recreate-active=\"true\"]');const fallback=document.querySelector(`[data-recreate-trigger=\"${{trigger}}\"]`);const surface=wrapper.current?.firstElementChild;if(!active||!fallback||!surface)return;const current=surface.style.translate;const a=active.getBoundingClientRect();const b=fallback.getBoundingClientRect();surface.style.translate=`${{a.right-b.right}}px ${{a.bottom-b.bottom}}px`;return()=>{{surface.style.translate=current}}}},[trigger]);return createPortal(<div ref={{wrapper}} className=\"recreateAnchoredSurface\">{{children}}</div>,document.body)}}\n{}\n",
        jsx_variants::selector()
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
