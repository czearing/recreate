use super::shadow_root;
use super::{
    jsx_attrs::{attributes, jsx_tag, quoted, void_tag},
    jsx_host_props::{adopted, binds_value},
    jsx_render_spacing::{
        leading_placeholder, leading_placeholder_extent, needs_text_space, placeholder,
        placeholder_extent, sibling_index,
    },
    stand_in,
    tree::Components,
};
use std::collections::BTreeMap;

pub fn component(component: &super::tree::Component, components: &Components) -> String {
    let Some(root) = component.roots.first() else {
        return String::new();
    };
    let Some(node) = components.nodes.get(root) else {
        return String::new();
    };
    let class = components.classes.get(root).cloned().unwrap_or_default();
    // The class is a default rather than a baked-in literal, so one component serves every
    // element of this tag however it is styled, and a call site that passes nothing still
    // renders exactly what it rendered before.
    let default = serde_json::to_string(&class).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        "import React from 'react';\nexport default function {}({{children,className={},...props}}){{return <{} className={{className}} {{...props}}>{{children}}</{}>}}\n",
        component.name,
        default,
        jsx_tag(&node.tag),
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
    let children = if binds_value(node) {
        String::new()
    } else {
        render_children(path, components, assets, depth + 1, handlers)
    };
    if shadow_root::is_root(node) {
        return shadow_root::element(node, &children, &indent);
    }
    if allow_component && let Some(index) = components.by_root.get(path) {
        let name = &components.items[*index].name;
        let class = components.classes.get(path).cloned().unwrap_or_default();
        let attributes = format!("{}{}", attributes(node, assets), adopted(path, components));
        return format!(
            "{indent}<{name} className={}{attributes}{}>\n{}{indent}</{name}>\n",
            quoted(&class),
            event(path, handlers),
            children
        );
    }
    let class = components.classes.get(path).cloned().unwrap_or_default();
    let attributes = format!("{}{}", attributes(node, assets), adopted(path, components));
    let tag = stand_in::tag(node, assets);
    if void_tag(tag) {
        return format!(
            "{indent}<{tag} className={}{}{} />\n",
            quoted(&class),
            attributes,
            event(path, handlers)
        );
    }
    format!(
        "{indent}<{tag} className={}{}{}>\n{}{indent}</{tag}>\n",
        quoted(&class),
        attributes,
        event(path, handlers),
        children,
    )
}

pub(super) fn render_children(
    path: &str,
    components: &Components,
    assets: &BTreeMap<String, String>,
    depth: usize,
    handlers: &BTreeMap<String, String>,
) -> String {
    let mut indexes = BTreeMap::<String, usize>::new();
    let mut output = String::new();
    let children = components.children.get(path).cloned().unwrap_or_default();
    if let Some(extent) = leading_placeholder_extent(path, &children, components) {
        output.push_str(&leading_placeholder(depth, extent));
    }
    let mut previous_text = None;
    for child in &children {
        let node = &components.nodes[child];
        if previous_text.is_some_and(|previous| needs_text_space(previous, node)) {
            output.push_str(&format!("{}{{\" \"}}\n", "  ".repeat(depth)));
        }
        if let Some((tag, index)) = sibling_index(child) {
            let previous = indexes.entry(tag.to_string()).or_default();
            let missing = index.saturating_sub(*previous + 1);
            let extent = placeholder_extent(path, child, missing, components);
            for _ in 0..missing {
                output.push_str(&placeholder(tag, depth, extent));
            }
            *previous = index;
        }
        output.push_str(&render(child, components, assets, depth, true, handlers));
        previous_text = (node.tag == "#text").then_some(node);
    }
    output
}

fn event(path: &str, handlers: &BTreeMap<String, String>) -> String {
    handlers
        .get(path)
        .map(|value| format!(" {value}"))
        .unwrap_or_default()
}
