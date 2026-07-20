use crate::model::{Pseudo, Specification, Styles};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

pub fn declarations(styles: &Styles, assets: &BTreeMap<String, String>) -> String {
    styles
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| {
            let value = if value.contains("url(") {
                assets
                    .iter()
                    .fold(value.clone(), |text, (url, local)| text.replace(url, local))
            } else {
                value.clone()
            };
            format!("{key}:{value};")
        })
        .collect()
}

pub fn responsive_signatures_for(
    specification: &Specification,
    paths: Option<&HashSet<String>>,
) -> BTreeMap<String, String> {
    let mut signatures = BTreeMap::<String, Sha256>::new();
    for state in &specification.states {
        for node in &state.nodes {
            if paths.is_some_and(|paths| !paths.contains(&node.path)) {
                continue;
            }
            let signature = signatures.entry(node.path.clone()).or_default();
            append_styles(signature, &node.style);
            append_pseudo(signature, node.before.as_ref());
            append_pseudo(signature, node.after.as_ref());
        }
    }
    signatures
        .into_iter()
        .map(|(path, signature)| (path, hex::encode(signature.finalize())))
        .collect()
}

fn append_styles(signature: &mut Sha256, styles: &Styles) {
    for (key, value) in styles {
        signature.update(key.as_bytes());
        signature.update([0]);
        signature.update(value.as_bytes());
        signature.update([0xff]);
    }
}

fn append_pseudo(signature: &mut Sha256, pseudo: Option<&Pseudo>) {
    signature.update([0xfe]);
    if let Some(pseudo) = pseudo {
        signature.update(pseudo.content.as_bytes());
        append_styles(signature, &pseudo.style);
    }
}

pub fn hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
