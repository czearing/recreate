use crate::model::{Pseudo, Specification, Styles};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub fn declarations(styles: &Styles, assets: &BTreeMap<String, String>) -> String {
    styles
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| {
            let value = assets
                .iter()
                .fold(value.clone(), |text, (url, local)| text.replace(url, local));
            format!("{key}:{value};")
        })
        .collect()
}

pub fn responsive_signature(specification: &Specification, path: &str) -> String {
    specification
        .states
        .iter()
        .filter_map(|state| state.nodes.iter().find(|node| node.path == path))
        .map(|node| {
            format!(
                "{}|{}|{}",
                style_signature(&node.style),
                pseudo_signature(node.before.as_ref()),
                pseudo_signature(node.after.as_ref())
            )
        })
        .collect::<Vec<_>>()
        .join("||")
}

pub fn style_signature(styles: &Styles) -> String {
    styles
        .iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<_>>()
        .join(";")
}

fn pseudo_signature(pseudo: Option<&Pseudo>) -> String {
    pseudo
        .map(|pseudo| format!("{}:{}", pseudo.content, style_signature(&pseudo.style)))
        .unwrap_or_default()
}

pub fn hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
