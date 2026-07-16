use super::{
    interactions,
    jsx_attrs::{all_attributes, dynamic_attributes, jsx_tag, quoted, static_attributes, void_tag},
    jsx_variants, structural_tree,
    tree::Components,
};
use crate::model::Specification;
use std::collections::BTreeMap;

pub fn app(
    specification: &Specification,
    components: &Components,
    class_maps: &[BTreeMap<String, String>],
    assets: &BTreeMap<String, String>,
) -> String {
    if specification.states.is_empty() {
        return "export default function App(){return null}\n".into();
    }
    let handlers = interactions::base_handlers(specification);
    let views = specification
        .states
        .iter()
        .zip(class_maps)
        .enumerate()
        .map(|(index, (state, classes))| {
            let current = structural_tree::for_state(components, state, classes);
            let page = jsx_variants::page(state, &current, assets, &handlers);
            format!("function Baseline{index}({{activate}}){{return {page}}}\n")
        })
        .collect::<String>();
    let view_names = (0..specification.states.len())
        .map(|index| format!("Baseline{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let widths = jsx_variants::widths(&specification.states);
    let state_imports = (1..=specification.interactions.len())
        .map(|index| format!("Interaction{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let state_branches = (1..=specification.interactions.len())
        .map(|index| {
            format!(
                "if(state==={index})return <Interaction{index} width={{width}} onReset={{reset}}/>;"
            )
        })
        .collect::<String>();
    format!(
        "import React,{{useRef,useState,useSyncExternalStore}} from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{ {} }} from './components/index.js';\nimport {{ {} }} from './states.jsx';\nconst keyActivate=(event,action)=>{{if(event.key==='Enter'||event.key===' '){{event.preventDefault();action(event)}}}};\n{}\nconst viewportWidths=[{widths}];\nconst subscribe=notify=>{{const media=viewportWidths.slice(1).map(width=>matchMedia(`(max-width:${{width}}px)`));media.forEach(query=>query.addEventListener('change',notify));addEventListener('resize',notify);return()=>{{media.forEach(query=>query.removeEventListener('change',notify));removeEventListener('resize',notify)}}}};\n{views}const baselineViews=[{view_names}];\nexport default function App(){{const[state,setState]=useState(0);const lastTrigger=useRef('');const width=useSyncExternalStore(subscribe,()=>document.documentElement.clientWidth,()=>0);const viewport=selectViewport(width,viewportWidths);const View=baselineViews[viewport];const activate=(event,next)=>{{lastTrigger.current=event.currentTarget.dataset.recreateTrigger;setState(next)}};const reset=()=>{{setState(0);requestAnimationFrame(()=>document.querySelector('[data-recreate-trigger=\"'+lastTrigger.current+'\"]')?.focus())}};{state_branches}return <View activate={{activate}}/>}}\n",
        components
            .items
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        state_imports,
        jsx_variants::selector(),
    )
}

pub fn component(
    component: &super::tree::Component,
    components: &Components,
    assets: &BTreeMap<String, String>,
) -> String {
    let Some(root) = component.roots.first() else {
        return String::new();
    };
    let Some(node) = components.nodes.get(root) else {
        return String::new();
    };
    let class = components.classes.get(root).cloned().unwrap_or_default();
    let attributes = static_attributes(node, assets);
    format!(
        "import React from 'react';\nexport default function {}({{children,...props}}){{return <{} className={}{} {{...props}}>{{children}}</{}>}}\n",
        component.name,
        jsx_tag(&node.tag),
        quoted(&class),
        attributes,
        jsx_tag(&node.tag)
    )
}

pub(super) fn render(
    path: &str,
    components: &Components,
    assets: &BTreeMap<String, String>,
    depth: usize,
    allow_component: bool,
    handlers: &BTreeMap<String, String>,
) -> String {
    let Some(node) = components.nodes.get(path) else {
        return String::new();
    };
    let indent = "  ".repeat(depth);
    if node.tag == "#text" {
        return format!(
            "{indent}{{{}}}\n",
            serde_json::to_string(&node.text).unwrap()
        );
    }
    let children = components
        .children
        .get(path)
        .into_iter()
        .flatten()
        .map(|child| render(child, components, assets, depth + 1, true, handlers))
        .collect::<String>();
    if allow_component && let Some(index) = components.by_root.get(path) {
        let name = &components.items[*index].name;
        let attributes = dynamic_attributes(node, assets);
        return format!(
            "{indent}<{name}{attributes}{}>\n{}{indent}</{name}>\n",
            event(path, handlers),
            children
        );
    }
    let class = components.classes.get(path).cloned().unwrap_or_default();
    let attributes = all_attributes(node, assets);
    if void_tag(&node.tag) {
        return format!(
            "{indent}<{} className={}{}{} />\n",
            jsx_tag(&node.tag),
            quoted(&class),
            attributes,
            event(path, handlers)
        );
    }
    format!(
        "{indent}<{} className={}{}{}>\n{}{indent}</{}>\n",
        jsx_tag(&node.tag),
        quoted(&class),
        attributes,
        event(path, handlers),
        children,
        jsx_tag(&node.tag)
    )
}

fn event(path: &str, handlers: &BTreeMap<String, String>) -> String {
    handlers
        .get(path)
        .map(|value| format!(" {value}"))
        .unwrap_or_default()
}
