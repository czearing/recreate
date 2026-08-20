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
    let resting: BTreeMap<String, String> = signatures
        .into_iter()
        .map(|(path, signature)| (path, signature.finish()))
        .collect();
    let mut folded = BTreeMap::<String, Signature>::new();
    for state in &specification.states {
        for style in &state.state_styles {
            append_state_style(&mut folded, &resting, style, assets);
        }
    }
    resting
        .iter()
        .map(|(path, bytes)| {
            let signature = folded
                .remove(path)
                .map_or_else(|| bytes.clone(), |signature: Signature| signature.finish());
            (path.clone(), signature)
        })
        .collect()
}

/// Folds in a rule an element receives only while the page is in some state.
///
/// An element's identity is the rules it will receive, and a state rule is one of them: two
/// elements that rest identically but answer focus differently are two elements, and giving
/// them one class publishes two rules with the same selector, of which only the last survives.
/// Both sides are folded, because a rule names two elements — the one it styles and the one
/// holding the state — and moving either changes what is emitted.
///
/// The counterpart is folded by its resting identity rather than by its path. A path is unique
/// per element, so folding one would split every reused component into as many classes as it
/// has instances, which is the collapse this whole mechanism exists to perform. A resting
/// identity is exactly the question being asked one level down: two rules differ when the
/// elements they join differ.
///
/// An element no state rule names is left out of the map entirely, so its identity stays the
/// resting digest untouched. Folding "nothing" into every element would be equally correct and
/// would rename every class on every page for no change in meaning.
fn append_state_style(
    signatures: &mut BTreeMap<String, Signature>,
    resting: &BTreeMap<String, String>,
    style: &crate::model::StateStyle,
    assets: &BTreeMap<String, String>,
) {
    let counterpart = |path: Option<&str>| {
        path.and_then(|path| resting.get(path))
            .map(String::as_str)
            .unwrap_or_default()
    };
    for (path, side, other) in [
        (
            style.target.as_str(),
            "target",
            counterpart(style.scope.as_deref()),
        ),
        (
            style.scope.as_deref().unwrap_or_default(),
            "scope",
            counterpart(Some(style.target.as_str())),
        ),
    ] {
        let Some(bytes) = resting.get(path) else {
            continue;
        };
        let signature = signatures.entry(path.to_string()).or_insert_with(|| {
            let mut signature = Signature::new();
            signature.value(bytes);
            signature
        });
        signature.slot();
        signature.value(side);
        signature.value(other);
        signature.value(style.relation.name());
        signature.pair("pseudo", style.pseudo.as_deref().unwrap_or_default());
        signature.pair(
            "target-pseudo",
            style.target_pseudo.as_deref().unwrap_or_default(),
        );
        signature.pair("media", style.media.as_deref().unwrap_or_default());
        signature.value(&super::asset_urls::rewrite(&style.declarations, assets));
    }
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
