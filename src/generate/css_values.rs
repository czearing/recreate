use super::css_signature::Signature;
use crate::model::{Pseudo, Specification, Styles};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

pub fn declarations(styles: &Styles, assets: &BTreeMap<String, String>) -> String {
    styles
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| {
            let value = super::asset_urls::rewrite(value, assets);
            format!("{key}:{value};")
        })
        .collect()
}

pub fn responsive_signatures_for(
    specification: &Specification,
    paths: Option<&HashSet<String>>,
    assets: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut signatures = BTreeMap::<String, Signature>::new();
    for state in &specification.states {
        for node in &state.nodes {
            if paths.is_some_and(|paths| !paths.contains(&node.path)) {
                continue;
            }
            let signature = signatures.entry(node.path.clone()).or_default();
            append_styles(signature, &node.style, assets);
            for (suffix, pseudo) in super::css_pseudo::slots(node) {
                append_pseudo(signature, suffix, pseudo, assets);
            }
        }
    }
    signatures
        .into_iter()
        .map(|(path, signature)| (path, signature.finish()))
        .collect()
}

/// Folds a style block in as the emitted rule will spell it. An element's identity is the
/// rules it will receive, and those rules are localised; folding the captured spelling
/// instead folds in the capture rig's ephemeral origin, which differs on every run.
fn append_styles(signature: &mut Signature, styles: &Styles, assets: &BTreeMap<String, String>) {
    for (key, value) in styles {
        signature.pair(key, &super::asset_urls::rewrite(value, assets));
    }
}

/// Writes the slot marker and which slot it was, so an element with no generated box is not
/// encoded as the same bytes as one whose box is empty, and two elements decorating different
/// slots with the same declarations do not collapse onto one class.
///
/// `content` is folded as the declaration it will become rather than as the captured field,
/// for the reason [`append_styles`] gives: an identity built from the captured spelling folds
/// the rig's ephemeral origin. Going through the emitter's own function is what makes the two
/// unable to drift apart.
fn append_pseudo(
    signature: &mut Signature,
    suffix: &str,
    pseudo: &Pseudo,
    assets: &BTreeMap<String, String>,
) {
    signature.slot();
    signature.value(suffix);
    signature.value(&super::css_pseudo::content_declaration(
        &pseudo.content,
        assets,
    ));
    append_styles(signature, &pseudo.style, assets);
}

pub fn hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

/// Add a class to a `class` attribute value, leaving one already present alone.
pub(super) fn append_class(value: &mut String, class: &str) {
    if value.split_whitespace().any(|item| item == class) {
        return;
    }
    if !value.is_empty() {
        value.push(' ');
    }
    value.push_str(class);
}
