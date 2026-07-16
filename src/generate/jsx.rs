use super::{
    jsx_attrs::{all_attributes, dynamic_attributes, jsx_tag, quoted, static_attributes, void_tag},
    tree::Components,
};
use crate::model::Specification;
use std::collections::BTreeMap;

pub fn app(
    specification: &Specification,
    components: &Components,
    assets: &BTreeMap<String, String>,
) -> String {
    let Some(state) = specification.states.first() else {
        return "export default function App(){return null}\n".into();
    };
    let body_path = state
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
        .or_else(|| state.nodes.iter().find(|node| node.tag == "body"))
        .map(|node| node.path.as_str())
        .unwrap_or("html");
    let handlers: BTreeMap<_, _> = specification
        .interactions
        .iter()
        .enumerate()
        .map(|(index, interaction)| {
            (
                interaction.trigger_path.clone(),
                format!("onClick={{()=>setState({})}}", index + 1),
            )
        })
        .collect();
    let content = components
        .children
        .get(root)
        .into_iter()
        .flatten()
        .map(|path| render(path, components, assets, 2, true, &handlers))
        .collect::<String>();
    let portals = if root == body_path {
        String::new()
    } else {
        components
            .children
            .get(body_path)
            .into_iter()
            .flatten()
            .filter(|path| path.as_str() != root)
            .map(|path| render(path, components, assets, 2, true, &handlers))
            .collect::<String>()
    };
    let portal = if portals.is_empty() {
        String::new()
    } else {
        format!("{{createPortal(<>{portals}</>,document.body)}}")
    };
    let state_imports = (1..=specification.interactions.len())
        .map(|index| format!("Interaction{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let state_branches = (1..=specification.interactions.len())
        .map(|index| {
            format!("if(state==={index})return <Interaction{index} onReset={{()=>setState(0)}}/>;")
        })
        .collect::<String>();
    format!(
        "import React,{{useState}} from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{ {} }} from './components/index.js';\nimport {{ {} }} from './states.jsx';\nexport default function App(){{const[state,setState]=useState(0);{state_branches}return <>{content}{portal}</>}}\n",
        components
            .items
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        state_imports
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
